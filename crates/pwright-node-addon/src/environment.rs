//! Node environment ownership, independent of JS object finalization order.
//! Finite admission covers weak sessions and their unpublished metadata endpoint.

#[cfg(feature = "playwright-trace")]
use std::path::Path;
#[cfg(feature = "owned-launch")]
use std::path::PathBuf;
#[cfg(any(
    feature = "cdp-default-user-agent",
    feature = "playwright-server-connect-headers",
    feature = "playwright-server-connect-session"
))]
use std::sync::OnceLock;
use std::sync::{Arc, Mutex, Weak};
#[cfg(any(
    feature = "playwright-server-connect-close-error",
    feature = "playwright-server-connect-session"
))]
use std::time::{SystemTime, UNIX_EPOCH};

use napi::{
    Env, JsString, Unknown,
    bindgen_prelude::{Function, PromiseRaw},
};
#[cfg(any(
    feature = "cdp-handshake-headers",
    feature = "playwright-server-connect-ws-error",
    feature = "playwright-server-connect-close-error",
    feature = "playwright-server-connect-headers",
    feature = "playwright-server-connect-session"
))]
use napi::{
    JsValue, ValueType,
    bindgen_prelude::{JsObjectValue, Object},
};
use napi_derive::napi;
use pwright_node_host::ffi_budget::{FfiBudget, FfiLane, FfiLimits, FfiMetrics};
use pwright_node_host::locator_query::{QueryBudget, QueryLimits};
#[cfg(feature = "owned-launch")]
use pwright_node_host::{ChromiumChannel, ChromiumExecutable};
use pwright_node_host::{HostError, HostSession, SessionMetrics};
#[cfg(feature = "bidi-startup-failure")]
use pwright_owned_process_startup::{StartupObservation, StartupReport, observe_explicit_startup};
#[cfg(feature = "playwright-server-connect-session")]
use pwright_playwright_server_session::RemoteBrowserSupervisor;
#[cfg(feature = "playwright-trace")]
use pwright_trace_playwright_v8::{TraceDirectoryLimits, prepare_trace_directory};
#[cfg(any(
    feature = "playwright-server-connect-ws-error",
    feature = "playwright-server-connect-headers"
))]
use pwright_transport_ws::LiveWsConnector;
#[cfg(any(
    feature = "playwright-server-connect-ws-error",
    feature = "playwright-server-connect-close-error",
    feature = "playwright-server-connect-headers"
))]
use pwright_transport_ws::WsError;
#[cfg(feature = "playwright-server-connect-session")]
use pwright_transport_ws::WsHeaders;
#[cfg(any(
    feature = "playwright-server-connect-ws-error",
    feature = "playwright-server-connect-close-error",
    feature = "playwright-server-connect-headers",
    feature = "playwright-server-connect-session"
))]
use pwright_transport_ws::{
    MAX_ENDPOINT_BYTES, MAX_HEADER_COUNT, MAX_HEADER_NAME_BYTES, MAX_HEADER_VALUE_BYTES,
    WsEndpoint, WsLimits, WsRequest,
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

use crate::deferred_conversion::{ConversionMetrics, DeferredConversions};
use crate::{NativeSession, binding_error, boundary};

const MAX_SESSIONS: usize = 16;

#[cfg(feature = "playwright-server-connect-session")]
fn map_server_connect_error(
    error: pwright_playwright_server_connect::ConnectError,
    budget_ms: u32,
) -> HostError {
    use pwright_playwright_server_connect::ConnectError;
    use pwright_playwright_server_endpoint_discovery::{DiscoveryError, FetchError};
    use pwright_playwright_server_session::SessionError;
    use pwright_transport_ws::WsError;

    if let Some(mismatch) = error.playwright_version_mismatch() {
        return HostError::Backend(format!(
            "Playwright version mismatch:\n  - server version: v{}\n  - client version: v{}",
            mismatch.server_version(),
            mismatch.client_version(),
        ));
    }

    if matches!(
        &error,
        ConnectError::Discovery(
            DiscoveryError::InvalidEndpoint(_)
                | DiscoveryError::InvalidLimits(_)
                | DiscoveryError::InvalidUserAgent(_)
        )
    ) {
        HostError::InvalidArgument(error.to_string())
    } else if matches!(
        &error,
        ConnectError::Discovery(DiscoveryError::Fetch(FetchError::Timeout))
            | ConnectError::WebSocket(WsError::ConnectTimeout)
            | ConnectError::Session(pwright_playwright_server_session::SessionFailure {
                error: SessionError::WebSocket(WsError::ConnectTimeout),
                ..
            })
            | ConnectError::Session(pwright_playwright_server_session::SessionFailure {
                error: SessionError::DeadlineExceeded,
                ..
            })
    ) {
        HostError::Timeout(budget_ms)
    } else {
        HostError::Backend(error.to_string())
    }
}
// Upstream getUserAgent() caches once per Node process. This opt-in cache is
// intentionally process-wide rather than tied to a NativeEnvironment wrapper.
#[cfg(any(
    feature = "cdp-default-user-agent",
    feature = "playwright-server-connect-headers",
    feature = "playwright-server-connect-session"
))]
static DEFAULT_PLAYWRIGHT_USER_AGENT: OnceLock<String> = OnceLock::new();

#[cfg(feature = "cdp-handshake-headers")]
fn bounded_cdp_handshake_headers(raw: Unknown<'_>) -> napi::Result<Vec<(String, String)>> {
    use pwright_node_host::{
        CDP_HANDSHAKE_HEADER_MAX_BATCH_BYTES, CDP_HANDSHAKE_HEADER_MAX_COUNT,
        CDP_HANDSHAKE_HEADER_MAX_NAME_BYTES, CDP_HANDSHAKE_HEADER_MAX_VALUE_BYTES,
    };

    if !raw.is_array()? {
        return Err(binding_error(HostError::InvalidArgument(
            "CDP handshake headers must be an array of pairs".into(),
        )));
    }
    // SAFETY: is_array checked above; all handles remain in the callback scope.
    let entries = unsafe { raw.cast::<Object<'_>>()? };
    let count = entries.get_array_length()? as usize;
    if count > CDP_HANDSHAKE_HEADER_MAX_COUNT {
        return Err(binding_error(HostError::ResourceExhausted(
            "CDP handshake header count exceeds 32".into(),
        )));
    }
    let mut headers = Vec::with_capacity(count);
    let mut bytes = 0usize;
    for index in 0..count {
        let pair: Unknown<'_> = entries.get_element(index as u32)?;
        if !pair.is_array()? {
            return Err(binding_error(HostError::InvalidArgument(
                "CDP handshake header must be a name/value pair".into(),
            )));
        }
        // SAFETY: is_array checked above; no JS handle escapes this callback.
        let pair = unsafe { pair.cast::<Object<'_>>()? };
        if pair.get_array_length()? != 2 {
            return Err(binding_error(HostError::InvalidArgument(
                "CDP handshake header must contain exactly two strings".into(),
            )));
        }
        let name: Unknown<'_> = pair.get_element(0)?;
        let value: Unknown<'_> = pair.get_element(1)?;
        if name.get_type()? != ValueType::String || value.get_type()? != ValueType::String {
            return Err(binding_error(HostError::InvalidArgument(
                "CDP handshake header name and value must be strings".into(),
            )));
        }
        // SAFETY: both types were checked above, before any owned Rust copy.
        let name = unsafe { name.cast::<JsString<'_>>()? };
        let value = unsafe { value.cast::<JsString<'_>>()? };
        let name_bytes = name.utf8_len()?;
        let value_bytes = value.utf8_len()?;
        if name_bytes > CDP_HANDSHAKE_HEADER_MAX_NAME_BYTES {
            return Err(binding_error(HostError::ResourceExhausted(
                "CDP handshake header name exceeds 256 bytes".into(),
            )));
        }
        if value_bytes > CDP_HANDSHAKE_HEADER_MAX_VALUE_BYTES {
            return Err(binding_error(HostError::ResourceExhausted(
                "CDP handshake header value exceeds 4096 bytes".into(),
            )));
        }
        bytes = bytes
            .checked_add(name_bytes)
            .and_then(|sum| sum.checked_add(value_bytes))
            .ok_or_else(|| {
                binding_error(HostError::ResourceExhausted(
                    "CDP handshake headers exceed 8192 bytes".into(),
                ))
            })?;
        if bytes > CDP_HANDSHAKE_HEADER_MAX_BATCH_BYTES {
            return Err(binding_error(HostError::ResourceExhausted(
                "CDP handshake headers exceed 8192 bytes".into(),
            )));
        }
        headers.push((
            name.into_utf8()?.into_owned()?,
            value.into_utf8()?.into_owned()?,
        ));
    }
    Ok(headers)
}

#[cfg(any(
    feature = "playwright-server-connect-ws-error",
    feature = "playwright-server-connect-close-error",
    feature = "playwright-server-connect-headers",
    feature = "playwright-server-connect-session"
))]
fn bounded_server_connect_headers_with(
    raw: Unknown<'_>,
    mut add: impl FnMut(&str, &str) -> napi::Result<()>,
) -> napi::Result<()> {
    if !raw.is_array()? {
        return Err(binding_error(HostError::InvalidArgument(
            "WebSocket headers must be an array of pairs".into(),
        )));
    }
    // SAFETY: is_array was checked; all handles remain in this callback scope.
    let entries = unsafe { raw.cast::<Object<'_>>()? };
    let count = entries.get_array_length()? as usize;
    if count > MAX_HEADER_COUNT {
        return Err(binding_error(HostError::ResourceExhausted(
            "WebSocket header count exceeds 64".into(),
        )));
    }
    for index in 0..count {
        let pair: Unknown<'_> = entries.get_element(index as u32)?;
        if !pair.is_array()? {
            return Err(binding_error(HostError::InvalidArgument(
                "WebSocket header must be a name/value pair".into(),
            )));
        }
        // SAFETY: is_array was checked; pair does not escape this callback.
        let pair = unsafe { pair.cast::<Object<'_>>()? };
        if pair.get_array_length()? != 2 {
            return Err(binding_error(HostError::InvalidArgument(
                "WebSocket header must contain exactly two strings".into(),
            )));
        }
        let name: Unknown<'_> = pair.get_element(0)?;
        let value: Unknown<'_> = pair.get_element(1)?;
        if name.get_type()? != ValueType::String || value.get_type()? != ValueType::String {
            return Err(binding_error(HostError::InvalidArgument(
                "WebSocket header name and value must be strings".into(),
            )));
        }
        // SAFETY: both value types were checked before conversion.
        let name = unsafe { name.cast::<JsString<'_>>()? };
        let value = unsafe { value.cast::<JsString<'_>>()? };
        let name = boundary::bounded_string(name, MAX_HEADER_NAME_BYTES, "WebSocket header name")?;
        let value =
            boundary::bounded_string(value, MAX_HEADER_VALUE_BYTES, "WebSocket header value")?;
        add(&name, &value)?;
    }
    Ok(())
}

#[cfg(any(
    feature = "playwright-server-connect-ws-error",
    feature = "playwright-server-connect-close-error",
    feature = "playwright-server-connect-headers",
    feature = "playwright-server-connect-session"
))]
fn bounded_server_connect_headers(raw: Unknown<'_>, request: &mut WsRequest) -> napi::Result<()> {
    bounded_server_connect_headers_with(raw, |name, value| {
        request
            .add_header(name, value)
            .map_err(|error| binding_error(HostError::InvalidArgument(error.to_string())))
    })
}

#[cfg(feature = "playwright-server-connect-session")]
fn bounded_server_connect_headers_for_http_discovery(
    raw: Unknown<'_>,
    headers: &mut WsHeaders,
) -> napi::Result<()> {
    bounded_server_connect_headers_with(raw, |name, value| {
        headers
            .push_for_http_discovery(name, value)
            .map_err(|error| binding_error(HostError::InvalidArgument(error.to_string())))
    })
}

#[cfg(any(
    feature = "playwright-server-connect-headers",
    feature = "playwright-server-connect-session"
))]
fn bounded_server_connect_runtime_identity<'env>(
    raw: Unknown<'env>,
) -> napi::Result<(JsString<'env>, JsString<'env>, JsString<'env>)> {
    if !raw.is_array()? {
        return Err(binding_error(HostError::InvalidArgument(
            "runtime identity must be a three-string tuple".into(),
        )));
    }
    // SAFETY: is_array was checked; all handles remain in the callback scope.
    let values = unsafe { raw.cast::<Object<'env>>()? };
    if values.get_array_length()? != 3 {
        return Err(binding_error(HostError::InvalidArgument(
            "runtime identity must contain platform, architecture, and release".into(),
        )));
    }
    let platform: Unknown<'env> = values.get_element(0)?;
    let arch: Unknown<'env> = values.get_element(1)?;
    let release: Unknown<'env> = values.get_element(2)?;
    if platform.get_type()? != ValueType::String
        || arch.get_type()? != ValueType::String
        || release.get_type()? != ValueType::String
    {
        return Err(binding_error(HostError::InvalidArgument(
            "runtime identity fields must be strings".into(),
        )));
    }
    // SAFETY: all three value types were checked before conversion.
    let platform = unsafe { platform.cast::<JsString<'env>>()? };
    let arch = unsafe { arch.cast::<JsString<'env>>()? };
    let release = unsafe { release.cast::<JsString<'env>>()? };
    if platform.utf8_len()? > 64 || arch.utf8_len()? > 64 || release.utf8_len()? > 128 {
        return Err(binding_error(HostError::ResourceExhausted(
            "runtime identity exceeds its field byte limits".into(),
        )));
    }
    Ok((platform, arch, release))
}

#[cfg(feature = "owned-launch")]
pub(crate) type LaunchProfile = Arc<Mutex<Option<tempfile::TempDir>>>;

#[derive(Default)]
struct OwnerState {
    sessions: Vec<SessionRecord>,
    #[cfg(feature = "playwright-server-connect-session")]
    remote_sessions: Vec<Weak<RemoteBrowserSupervisor>>,
    pending: usize,
}

#[derive(Clone)]
struct SessionRecord {
    host: Weak<HostSession>,
    #[cfg(feature = "owned-launch")]
    _launch_profile: Option<LaunchProfile>,
    publications: pwright_node_host::FramePublications,
    // A finalized JS wrapper may drop the last HostSession Arc before its
    // background driver finishes. Keep only its completion signal, not the
    // session, so environment teardown can still join that native I/O drain.
    retired: CancellationToken,
}

struct EnvironmentOwner {
    stopped: CancellationToken,
    state: Mutex<OwnerState>,
    admissions: TaskTracker,
    ffi: FfiBudget,
    #[cfg(feature = "handles")]
    wait_admissions: crate::wait_admission::WaitAdmissions,
    queries: QueryBudget,
    #[cfg(feature = "selector-registration")]
    selectors: pwright_node_host::selector_registry::SelectorRegistry,
}

impl EnvironmentOwner {
    async fn stop_and_drain(&self) {
        self.stopped.cancel();
        if let Err(error) = self.ffi.shutdown().await {
            eprintln!("Pwright environment cleanup failed: FFI drain {error}");
        }
        match self.ffi.metrics() {
            Ok(metrics) if metrics == FfiMetrics::default() => {}
            other => eprintln!("Pwright environment cleanup failed: retained FFI state {other:?}"),
        }
        #[cfg(feature = "selector-registration")]
        if let Err(error) = self.selectors.clear() {
            eprintln!("Pwright environment cleanup failed: selector registry {error}");
        }
        // Cancel pending connects before taking the final published snapshot.
        // No JavaScript callbacks participate in this barrier.
        self.admissions.close();
        self.admissions.wait().await;
        #[cfg(feature = "playwright-server-connect-session")]
        let remote_sessions: Vec<_> = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remote_sessions
            .iter()
            .filter_map(|session| session.upgrade().filter(|remote| remote.is_connected()))
            .collect();
        #[cfg(feature = "playwright-server-connect-session")]
        if !remote_sessions.is_empty() {
            let mut closes = tokio::task::JoinSet::new();
            for remote in remote_sessions {
                closes.spawn(async move { remote.close().await });
            }
            let drain = async {
                while let Some(result) = closes.join_next().await {
                    match result {
                        Ok(Ok(_)) => {}
                        Ok(Err(error)) => eprintln!(
                            "Pwright environment cleanup failed: remote Browser close {error:?}"
                        ),
                        Err(error) => eprintln!(
                            "Pwright environment cleanup failed: remote Browser task {error}"
                        ),
                    }
                }
            };
            if tokio::time::timeout(std::time::Duration::from_secs(32), drain)
                .await
                .is_err()
            {
                eprintln!("Pwright environment cleanup failed: remote Browser close timed out");
            }
        }
        let sessions: Vec<_> = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .sessions
            .to_vec();
        for session in &sessions {
            session.publications.discard();
            if let Some(host) = session.host.upgrade() {
                host.request_shutdown();
            }
        }
        for record in sessions {
            let Some(session) = record.host.upgrade() else {
                record.retired.cancelled().await;
                continue;
            };
            if let Err(error) = session.close().await {
                // JavaScript execution is already disabled during teardown.
                // Keep cleanup failures visible to embedders/test diagnostics.
                eprintln!("Pwright environment cleanup failed: {error}");
            }
            match session.metrics() {
                Ok(metrics) if metrics == SessionMetrics::default() => {}
                Ok(metrics) => eprintln!(
                    "Pwright environment cleanup failed: retained native resources {metrics:?}"
                ),
                Err(error) => eprintln!("Pwright environment cleanup failed: {error}"),
            }
        }
    }

    fn admit(self: &Arc<Self>) -> Result<Admission, HostError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| HostError::Backend("Node environment ownership is unavailable".into()))?;
        if self.stopped.is_cancelled() {
            return Err(HostError::TargetClosed);
        }
        let mut index = 0;
        while index < state.sessions.len() {
            let session = &state.sessions[index];
            if session.retired.is_cancelled() && session.publications.is_drained()? {
                state.sessions.swap_remove(index);
            } else {
                index += 1;
            }
        }
        #[cfg(feature = "playwright-server-connect-session")]
        state.remote_sessions.retain(|session| {
            session
                .upgrade()
                .is_some_and(|remote| remote.is_connected())
        });
        #[cfg(feature = "playwright-server-connect-session")]
        let remote_count = state.remote_sessions.len();
        #[cfg(not(feature = "playwright-server-connect-session"))]
        let remote_count = 0;
        if state.sessions.len() + remote_count + state.pending >= MAX_SESSIONS {
            return Err(HostError::ResourceExhausted(
                "Node environment session capacity exceeded (16)".into(),
            ));
        }
        state.pending += 1;
        Ok(Admission {
            owner: Arc::clone(self),
            pending: true,
            _task: self.admissions.token(),
        })
    }
}

struct Admission {
    owner: Arc<EnvironmentOwner>,
    pending: bool,
    _task: tokio_util::task::task_tracker::TaskTrackerToken,
}

impl Admission {
    fn publish(
        mut self,
        session: &Arc<HostSession>,
        #[cfg(feature = "owned-launch")] launch_profile: Option<LaunchProfile>,
    ) -> Result<(), HostError> {
        let mut state =
            self.owner.state.lock().map_err(|_| {
                HostError::Backend("Node environment ownership is unavailable".into())
            })?;
        state.pending -= 1;
        self.pending = false;
        state.sessions.push(SessionRecord {
            host: Arc::downgrade(session),
            #[cfg(feature = "owned-launch")]
            _launch_profile: launch_profile,
            publications: session.frame_publications(),
            retired: session.disconnect_signal(),
        });
        if self.owner.stopped.is_cancelled() {
            session.request_shutdown();
            return Err(HostError::TargetClosed);
        }
        Ok(())
    }

    #[cfg(feature = "playwright-server-connect-session")]
    fn publish_remote(mut self, remote: &Arc<RemoteBrowserSupervisor>) -> Result<(), HostError> {
        let mut state =
            self.owner.state.lock().map_err(|_| {
                HostError::Backend("Node environment ownership is unavailable".into())
            })?;
        state.pending -= 1;
        self.pending = false;
        state.remote_sessions.push(Arc::downgrade(remote));
        if self.owner.stopped.is_cancelled() {
            return Err(HostError::TargetClosed);
        }
        Ok(())
    }
}

impl Drop for Admission {
    fn drop(&mut self) {
        if self.pending {
            let mut state = self
                .owner
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.pending -= 1;
        }
    }
}

#[napi]
pub struct NativeEnvironment {
    #[cfg(feature = "retained-file-input")]
    pub(crate) file_input_timeouts: pwright_node_host::handles::TimeoutPolicyBudget,
    #[cfg(feature = "retained-file-input")]
    file_input_preparations: crate::handles::file_input::FileInputPreparations,
    owner: Arc<EnvironmentOwner>,
    publications: crate::publication_port::PublicationPorts,
    conversions: DeferredConversions,
    locator_conversions: crate::locator_conversion::LocatorConversions,
}

#[napi]
impl NativeEnvironment {
    #[napi(constructor)]
    pub fn new(env: Env, scheduler: Function<'_>, maximum: Function<'_>) -> napi::Result<Self> {
        #[cfg(feature = "retained-file-input")]
        let file_input_timeouts =
            pwright_node_host::handles::TimeoutPolicyBudget::new().map_err(binding_error)?;
        #[cfg(feature = "retained-file-input")]
        let file_input_preparations =
            crate::handles::file_input::FileInputPreparations::new().map_err(binding_error)?;
        let publications =
            crate::publication_port::PublicationPorts::new().map_err(binding_error)?;
        let conversions = DeferredConversions::new(&env, scheduler, maximum)?;
        let locator_conversions = crate::locator_conversion::LocatorConversions::new(
            &env,
            scheduler,
            conversions.clone(),
        )?;
        #[cfg(feature = "large-evaluation-results")]
        // Ordinary calls retain their normal small admission. Exactly one
        // selected large result may add this allowance after Rust knows that
        // the returned value crossed the inline boundary.
        let ffi_reserved_bytes =
            64 * 1024 * 1024 + pwright_node_host::evaluation::LARGE_RESULT_FFI_ALLOWANCE;
        #[cfg(not(feature = "large-evaluation-results"))]
        let ffi_reserved_bytes = 64 * 1024 * 1024;
        let owner = Arc::new(EnvironmentOwner {
            queries: QueryBudget::new(QueryLimits {
                recipes: 1024,
                bytes: 8 * 1024 * 1024,
            })
            .map_err(binding_error)?,
            stopped: CancellationToken::new(),
            state: Mutex::new(OwnerState::default()),
            admissions: TaskTracker::new(),
            ffi: FfiBudget::new(FfiLimits {
                connects: 16,
                #[cfg(feature = "handles")]
                commands: crate::wait_admission::COMMAND_CAPACITY,
                #[cfg(not(feature = "handles"))]
                commands: 32,
                closes: 16,
                watches: 16,
                reserved_bytes: ffi_reserved_bytes,
                future_bytes: [1024 * 1024, 128 * 1024, 16 * 1024, 16 * 1024],
            })
            .map_err(binding_error)?,
            #[cfg(feature = "handles")]
            wait_admissions: crate::wait_admission::WaitAdmissions::new(),
            #[cfg(feature = "selector-registration")]
            selectors: pwright_node_host::selector_registry::SelectorRegistry::default(),
        });
        // One hook for the lazily constructed facade environment, not a hook
        // per attach. Node runs it even when pending Promises retain wrappers.
        // Cleanup hooks run in reverse registration order. napi registers its
        // runtime teardown while loading the addon, before this constructor.
        // Finish native I/O here before that earlier hook destroys the runtime;
        // merely spawning cleanup would race last-Worker runtime destruction.
        env.add_env_cleanup_hook(
            (
                Arc::clone(&owner),
                conversions.clone(),
                locator_conversions.clone(),
                publications.clone(),
            ),
            |(owner, conversions, locator_conversions, publications)| {
                publications.shutdown();
                locator_conversions.shutdown();
                conversions.shutdown();
                if conversions.metrics() != ConversionMetrics::default() {
                    eprintln!(
                        "Pwright environment cleanup failed: retained Node conversion references"
                    );
                }
                napi::bindgen_prelude::block_on(owner.stop_and_drain());
            },
        )?;
        // Cleanup hooks are LIFO. Seal policy lookup and preparation scopes
        // before deleting their Node roots, including finalizer reentry. The
        // earlier hook repeats idempotent publication shutdown, then joins I/O.
        #[cfg(feature = "retained-file-input")]
        env.add_env_cleanup_hook(
            (publications.clone(), file_input_preparations.clone()),
            |(publications, registry)| {
                registry.seal();
                publications.shutdown();
                registry.shutdown();
            },
        )?;
        Ok(Self {
            #[cfg(feature = "retained-file-input")]
            file_input_timeouts,
            #[cfg(feature = "retained-file-input")]
            file_input_preparations,
            owner,
            publications,
            conversions,
            locator_conversions,
        })
    }

    #[napi]
    pub fn register_selector_engine(
        &self,
        name: JsString<'_>,
        source: JsString<'_>,
        content_script: bool,
    ) -> napi::Result<()> {
        if self.owner.stopped.is_cancelled() {
            return Err(binding_error(HostError::TargetClosed));
        }
        #[cfg(feature = "selector-registration")]
        {
            let name = boundary::bounded_string(
                name,
                pwright_node_host::selector_registry::MAX_SELECTOR_ENGINE_NAME_BYTES,
                "selector engine name",
            )?;
            let source = boundary::bounded_string(
                source,
                pwright_node_host::selector_registry::MAX_SELECTOR_ENGINE_SOURCE_BYTES,
                "selector engine source",
            )?;
            self.owner
                .selectors
                .register(&name, &source, content_script)
                .map_err(binding_error)
        }
        #[cfg(not(feature = "selector-registration"))]
        {
            let _ = (name, source, content_script);
            Err(binding_error(HostError::UnsupportedOperation(
                "selector registration capability is not installed".into(),
            )))
        }
    }

    #[napi]
    pub fn set_test_id_attribute(&self, name: JsString<'_>) -> napi::Result<()> {
        if self.owner.stopped.is_cancelled() {
            return Err(binding_error(HostError::TargetClosed));
        }
        #[cfg(feature = "selector-registration")]
        {
            let name = boundary::bounded_string(
                name,
                pwright_node_host::selector_registry::MAX_TEST_ID_ATTRIBUTE_BYTES,
                "test-id attribute",
            )?;
            self.owner
                .selectors
                .set_test_id_attribute(&name)
                .map_err(binding_error)
        }
        #[cfg(not(feature = "selector-registration"))]
        {
            let _ = name;
            Err(binding_error(HostError::UnsupportedOperation(
                "test-id selector configuration capability is not installed".into(),
            )))
        }
    }

    #[napi(js_name = "connectOverCDP")]
    pub fn connect_over_cdp<'env>(
        &self,
        env: &'env Env,
        endpoint: JsString<'env>,
        timeout_ms: u32,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        self.connect_over_cdp_impl(
            env,
            endpoint,
            timeout_ms,
            None,
            #[cfg(feature = "cdp-default-user-agent")]
            None,
        )
    }
}

#[cfg(feature = "cdp-handshake-headers")]
#[napi]
impl NativeEnvironment {
    #[napi(js_name = "connectOverCDPWithHeaders")]
    pub fn connect_over_cdp_with_headers<'env>(
        &self,
        env: &'env Env,
        endpoint: JsString<'env>,
        timeout_ms: u32,
        headers: Unknown<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        self.connect_over_cdp_impl(
            env,
            endpoint,
            timeout_ms,
            Some(headers),
            #[cfg(feature = "cdp-default-user-agent")]
            None,
        )
    }
}

#[cfg(feature = "cdp-default-user-agent")]
#[napi]
impl NativeEnvironment {
    #[napi(js_name = "connectOverCDPWithDefaultUserAgent")]
    pub fn connect_over_cdp_with_default_user_agent<'env>(
        &self,
        env: &'env Env,
        endpoint: JsString<'env>,
        timeout_ms: u32,
        headers: Unknown<'env>,
        platform: JsString<'env>,
        arch: JsString<'env>,
        release: JsString<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        self.connect_over_cdp_impl(
            env,
            endpoint,
            timeout_ms,
            Some(headers),
            Some((platform, arch, release)),
        )
    }
}

impl NativeEnvironment {
    fn connect_over_cdp_impl<'env>(
        &self,
        env: &'env Env,
        endpoint: JsString<'env>,
        timeout_ms: u32,
        headers: Option<Unknown<'env>>,
        #[cfg(feature = "cdp-default-user-agent")] runtime_identity: Option<(
            JsString<'env>,
            JsString<'env>,
            JsString<'env>,
        )>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .owner
            .ffi
            .admit(FfiLane::Connect, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let endpoint = boundary::bounded_string(endpoint, 8192, "CDP endpoint")?;
        #[cfg(feature = "cdp-handshake-headers")]
        let headers = headers.map(bounded_cdp_handshake_headers).transpose()?;
        #[cfg(not(feature = "cdp-handshake-headers"))]
        let headers: Option<Vec<(String, String)>> = {
            let _ = headers;
            None
        };
        #[cfg(feature = "cdp-default-user-agent")]
        let headers = if let Some((platform, arch, release)) = runtime_identity {
            use pwright_node_host::cdp_default_user_agent::{
                format_user_agent, has_explicit_user_agent, validate_explicit_headers,
                with_default_user_agent,
            };
            let headers = headers.unwrap_or_default();
            validate_explicit_headers(&headers).map_err(binding_error)?;
            let default_user_agent = if has_explicit_user_agent(&headers) {
                String::new()
            } else if let Some(cached) = DEFAULT_PLAYWRIGHT_USER_AGENT.get() {
                cached.clone()
            } else {
                let inputs = crate::user_agent_runtime::collect(env, platform, arch, release)?;
                let computed = format_user_agent(&inputs);
                DEFAULT_PLAYWRIGHT_USER_AGENT
                    .get_or_init(|| computed)
                    .clone()
            };
            Some(with_default_user_agent(headers, &default_user_agent))
        } else {
            headers
        };
        let admission = self.owner.admit().map_err(binding_error)?;
        let owner = Arc::clone(&self.owner);
        let ffi = self.owner.ffi.clone();
        #[cfg(feature = "handles")]
        let wait_admissions = self.owner.wait_admissions.clone();
        let queries = self.owner.queries.clone();
        #[cfg(feature = "selector-registration")]
        let selectors = self.owner.selectors.clone();
        let conversions = self.conversions.clone();
        let locator_conversions = self.locator_conversions.clone();
        let publications = self.publications.clone();
        #[cfg(feature = "retained-file-input")]
        let file_input_timeout_budget = self.file_input_timeouts.clone();
        #[cfg(feature = "retained-file-input")]
        let file_input_preparations = self.file_input_preparations.clone();
        let connect = async move {
            #[cfg(feature = "cdp-handshake-headers")]
            if let Some(headers) = headers {
                return HostSession::connect_over_cdp_with_lifecycle_events_and_headers(
                    &endpoint, timeout_ms, headers,
                )
                .await;
            }
            #[cfg(not(feature = "cdp-handshake-headers"))]
            let _ = headers;
            HostSession::connect_over_cdp_with_lifecycle_events(&endpoint, timeout_ms).await
        };
        boundary::spawn_with_env(
            env,
            execution,
            async move {
                let host = tokio::select! {
                    biased;
                    () = owner.stopped.cancelled() => return Err(HostError::TargetClosed),
                    result = connect => Arc::new(result?),
                };
                admission.publish(
                    &host,
                    #[cfg(feature = "owned-launch")]
                    None,
                )?;
                Ok(host)
            },
            move |env, host| {
                #[cfg(feature = "retained-file-input")]
                let file_input_scope = match file_input_preparations.scope() {
                    Ok(scope) => scope,
                    Err(error) => {
                        host.frame_publications().discard();
                        host.request_shutdown();
                        return Err(binding_error(error));
                    }
                };
                let port = (|| {
                    let port = publications.admit(&host).map_err(binding_error)?;
                    #[cfg(feature = "retained-file-input")]
                    port.bind_file_input_scope(file_input_scope.clone());
                    port.publish(
                        env,
                        host.frame_publications().fence().map_err(binding_error)?,
                    )?;
                    Ok::<_, napi::Error>(port)
                })();
                let port = match port {
                    Ok(port) => port,
                    Err(error) => {
                        host.frame_publications().discard();
                        host.request_shutdown();
                        return Err(error);
                    }
                };
                Ok(NativeSession {
                    #[cfg(feature = "owned-launch")]
                    launch_profile: None,
                    #[cfg(feature = "retained-file-input")]
                    file_input_timeout_budget,
                    #[cfg(feature = "retained-file-input")]
                    file_input_scope,
                    #[cfg(feature = "retained-file-input")]
                    file_input_preparations,
                    publications: port,
                    host,
                    ffi,
                    #[cfg(feature = "handles")]
                    wait_admissions,
                    conversions,
                    locator_conversions,
                    queries,
                    #[cfg(feature = "selector-registration")]
                    selectors,
                })
            },
        )
    }
}

/// A bounded, opt-in diagnostic path for BrowserType.connect. No HostSession
/// is published even when the WebSocket upgrade succeeds.
#[cfg(feature = "playwright-server-connect-ws-error")]
#[napi]
impl NativeEnvironment {
    #[napi(js_name = "connectPlaywrightServerError")]
    pub fn connect_playwright_server_error<'env>(
        &self,
        env: &'env Env,
        endpoint: JsString<'env>,
        timeout_ms: u32,
        headers: Unknown<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .owner
            .ffi
            .admit(FfiLane::Connect, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let endpoint =
            boundary::bounded_string(endpoint, MAX_ENDPOINT_BYTES, "WebSocket endpoint")?;
        let endpoint = WsEndpoint::parse(endpoint)
            .map_err(|error| binding_error(HostError::InvalidArgument(error.to_string())))?;
        let public_endpoint = endpoint.without_query().to_owned();
        let limits = WsLimits::new(64 * 1024, 64 * 1024, timeout_ms)
            .map_err(|error| binding_error(HostError::InvalidArgument(error.to_string())))?;
        let mut request = WsRequest::new(endpoint, limits);
        bounded_server_connect_headers(headers, &mut request)?;
        let owner = Arc::clone(&self.owner);
        boundary::spawn_with_env(
            env,
            execution,
            async move {
                let result = tokio::select! {
                    biased;
                    () = owner.stopped.cancelled() => return Err(HostError::TargetClosed),
                    result = LiveWsConnector.connect_with_dns_classification(&request) => result,
                };
                let error = match result {
                    Ok(peer) => {
                        drop(peer);
                        HostError::UnsupportedOperation(
                            "remote Playwright Browser sessions are not implemented".into(),
                        )
                    }
                    Err(WsError::ConnectTimeout) => HostError::Timeout(timeout_ms),
                    Err(error) => HostError::Backend(format!(
                        "<ws connecting> {public_endpoint}\n<ws error> {error}"
                    )),
                };
                Err::<bool, HostError>(error)
            },
            |_, result| Ok(result),
        )
    }
}

/// A separate opt-in BrowserType.connect request-header observation. Only the
/// bounded WebSocket upgrade is performed; no remote Browser is published.
#[cfg(feature = "playwright-server-connect-headers")]
#[napi]
impl NativeEnvironment {
    #[napi(js_name = "connectPlaywrightServerHeaders")]
    pub fn connect_playwright_server_headers<'env>(
        &self,
        env: &'env Env,
        endpoint: JsString<'env>,
        timeout_ms: u32,
        headers: Unknown<'env>,
        browser_name: JsString<'env>,
        runtime_identity: Unknown<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        use pwright_playwright_connect_headers::{
            append_server_connect_defaults, format_user_agent, needs_default_user_agent,
        };

        let execution = self
            .owner
            .ffi
            .admit(FfiLane::Connect, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let endpoint =
            boundary::bounded_string(endpoint, MAX_ENDPOINT_BYTES, "WebSocket endpoint")?;
        let endpoint = WsEndpoint::parse(endpoint)
            .map_err(|error| binding_error(HostError::InvalidArgument(error.to_string())))?;
        let public_endpoint = endpoint.without_query().to_owned();
        let browser_name = boundary::bounded_string(browser_name, 16, "Browser name")?;
        let (platform, arch, release) = bounded_server_connect_runtime_identity(runtime_identity)?;
        let limits = WsLimits::new(64 * 1024, 64 * 1024, timeout_ms)
            .map_err(|error| binding_error(HostError::InvalidArgument(error.to_string())))?;
        let mut request = WsRequest::new(endpoint, limits);
        bounded_server_connect_headers(headers, &mut request)?;
        let default_user_agent = if needs_default_user_agent(&request) {
            if let Some(cached) = DEFAULT_PLAYWRIGHT_USER_AGENT.get() {
                cached.clone()
            } else {
                let inputs = crate::user_agent_runtime::collect(env, platform, arch, release)?;
                DEFAULT_PLAYWRIGHT_USER_AGENT
                    .get_or_init(|| format_user_agent(&inputs))
                    .clone()
            }
        } else {
            String::new()
        };
        append_server_connect_defaults(&mut request, &browser_name, &default_user_agent).map_err(
            |error| {
                let error = match error {
                    WsError::LimitExceeded { .. } | WsError::AllocationFailed => {
                        HostError::ResourceExhausted(error.to_string())
                    }
                    _ => HostError::InvalidArgument(error.to_string()),
                };
                binding_error(error)
            },
        )?;
        let owner = Arc::clone(&self.owner);
        boundary::spawn_with_env(
            env,
            execution,
            async move {
                let result = tokio::select! {
                    biased;
                    () = owner.stopped.cancelled() => return Err(HostError::TargetClosed),
                    result = LiveWsConnector.connect_with_dns_classification(&request) => result,
                };
                let error = match result {
                    Ok(peer) => {
                        drop(peer);
                        HostError::UnsupportedOperation(
                            "remote Playwright Browser sessions are not implemented".into(),
                        )
                    }
                    Err(WsError::ConnectTimeout) => HostError::Timeout(timeout_ms),
                    Err(error) => HostError::Backend(format!(
                        "<ws connecting> {public_endpoint}\n<ws error> {error}"
                    )),
                };
                Err::<bool, HostError>(error)
            },
            |_, result| Ok(result),
        )
    }
}

/// A separate opt-in failure observation for BrowserType.connect. The Rust
/// leaf sends Root.initialize and returns a remote Close; no HostSession or
/// remote Browser object graph is ever published.
#[cfg(feature = "playwright-server-connect-close-error")]
#[napi]
impl NativeEnvironment {
    #[napi(js_name = "connectPlaywrightServerCloseError")]
    pub fn connect_playwright_server_close_error<'env>(
        &self,
        env: &'env Env,
        endpoint: JsString<'env>,
        timeout_ms: u32,
        headers: Unknown<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        use pwright_playwright_server_initialize::{
            InitializeError, InitializeObservation, MAX_INITIALIZE_BUDGET_MS, SdkLanguage,
            observe_root_initialize_request,
        };

        let execution = self
            .owner
            .ffi
            .admit(FfiLane::Connect, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let endpoint =
            boundary::bounded_string(endpoint, MAX_ENDPOINT_BYTES, "WebSocket endpoint")?;
        let endpoint = WsEndpoint::parse(endpoint)
            .map_err(|error| binding_error(HostError::InvalidArgument(error.to_string())))?;
        let budget_ms = timeout_ms.min(MAX_INITIALIZE_BUDGET_MS);
        let limits = WsLimits::new(256 * 1024, 4096, budget_ms)
            .map_err(|error| binding_error(HostError::InvalidArgument(error.to_string())))?;
        let mut request = WsRequest::new(endpoint, limits);
        bounded_server_connect_headers(headers, &mut request)?;
        let wall_time_ms: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| {
                binding_error(HostError::Backend(
                    "system clock precedes the Unix epoch".into(),
                ))
            })?
            .as_millis()
            .try_into()
            .map_err(|_| {
                binding_error(HostError::Backend(
                    "system clock milliseconds exceed the supported range".into(),
                ))
            })?;
        let owner = Arc::clone(&self.owner);
        boundary::spawn_with_env(
            env,
            execution,
            async move {
                let observed = tokio::select! {
                    biased;
                    () = owner.stopped.cancelled() => return Err(HostError::TargetClosed),
                    result = observe_root_initialize_request(
                        &pwright_playwright_server_initialize::LiveWsConnector,
                        &request,
                        SdkLanguage::Javascript,
                        wall_time_ms,
                    ) => result,
                };
                let error = match observed {
                    Ok(InitializeObservation::RemoteClose(Some(frame)))
                        if !frame.reason().is_empty() =>
                    {
                        HostError::Backend(format!(
                            "{} (WebSocket close code {})",
                            frame.reason(),
                            frame.code()
                        ))
                    }
                    Ok(InitializeObservation::RemoteClose(Some(frame))) => {
                        HostError::Backend(format!("WebSocket closed with code {}", frame.code()))
                    }
                    Ok(InitializeObservation::RemoteClose(None)) => {
                        HostError::Backend("WebSocket closed without a close frame".into())
                    }
                    Ok(InitializeObservation::RootResponse)
                    | Ok(InitializeObservation::RemoteError { .. })
                    | Ok(InitializeObservation::UnsupportedProtocol) => {
                        HostError::UnsupportedOperation(
                            "remote Playwright Browser sessions are not implemented".into(),
                        )
                    }
                    Err(InitializeError::DeadlineExceeded)
                    | Err(InitializeError::WebSocket(WsError::ConnectTimeout)) => {
                        HostError::Timeout(budget_ms)
                    }
                    Err(error) => HostError::Backend(format!("WebSocket error: {error}")),
                };
                Err::<bool, HostError>(error)
            },
            |_, result| Ok(result),
        )
    }
}

/// A real, opt-in Playwright Server connection. It publishes a connection-
/// owned remote Browser lease only after Root.initialize and bootstrap
/// validation; it never reuses the Chromium CDP HostSession.
#[cfg(feature = "playwright-server-connect-session")]
struct RemoteConnectInputs<'env> {
    endpoint: JsString<'env>,
    timeout_ms: u32,
    headers: Unknown<'env>,
    browser_name: JsString<'env>,
    runtime_identity: Unknown<'env>,
}

#[cfg(feature = "playwright-server-connect-session")]
#[napi]
impl NativeEnvironment {
    #[napi(js_name = "connectPlaywrightServer")]
    pub fn connect_playwright_server<'env>(
        &self,
        env: &'env Env,
        endpoint: JsString<'env>,
        timeout_ms: u32,
        headers: Unknown<'env>,
        browser_name: JsString<'env>,
        runtime_identity: Unknown<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        self.connect_playwright_server_impl(
            env,
            RemoteConnectInputs {
                endpoint,
                timeout_ms,
                headers,
                browser_name,
                runtime_identity,
            },
            pwright_playwright_server_session::EventPolicy::DropUnsupportedEvents,
        )
    }
}

/// Keep optional Node-API methods in a feature-gated impl: napi-derive emits
/// callbacks for every method in an impl even when an individual method has cfg.
#[cfg(feature = "playwright-server-connect-page-events")]
#[napi]
impl NativeEnvironment {
    /// A separately selected, typed Page notice stream; the legacy port keeps
    /// its drop-unsupported policy even in an addon built with this feature.
    #[napi(js_name = "connectPlaywrightServerPageEvents")]
    pub fn connect_playwright_server_page_events<'env>(
        &self,
        env: &'env Env,
        endpoint: JsString<'env>,
        timeout_ms: u32,
        headers: Unknown<'env>,
        browser_name: JsString<'env>,
        runtime_identity: Unknown<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        self.connect_playwright_server_impl(
            env,
            RemoteConnectInputs {
                endpoint,
                timeout_ms,
                headers,
                browser_name,
                runtime_identity,
            },
            pwright_playwright_server_session::EventPolicy::ProjectPageNotices,
        )
    }
}

#[cfg(feature = "playwright-server-connect-session")]
impl NativeEnvironment {
    fn connect_playwright_server_impl<'env>(
        &self,
        env: &'env Env,
        inputs: RemoteConnectInputs<'env>,
        event_policy: pwright_playwright_server_session::EventPolicy,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        use pwright_playwright_connect_headers::{
            append_server_connect_defaults, append_server_connect_defaults_for_http_discovery,
            format_user_agent,
        };
        use pwright_playwright_server_connect::{ConnectOptions, connect_live_with_event_policy};
        use pwright_playwright_server_endpoint_discovery::DiscoveryLimits;
        use pwright_playwright_server_session::SdkLanguage;

        let RemoteConnectInputs {
            endpoint,
            timeout_ms,
            headers,
            browser_name,
            runtime_identity,
        } = inputs;

        let execution = self
            .owner
            .ffi
            .admit(FfiLane::Connect, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let endpoint =
            boundary::bounded_string(endpoint, MAX_ENDPOINT_BYTES, "Playwright Server endpoint")?;
        // WHATWG URL parsing trims surrounding C0 controls and ASCII space.
        // The upstream run-server test fixture includes a trailing newline in
        // its stdout-derived endpoint. Normalize only the outside here; the
        // strict endpoint parser still rejects embedded controls and userinfo.
        let endpoint = endpoint
            .trim_matches(|character: char| character <= '\u{0020}')
            .to_owned();
        let browser_name = boundary::bounded_string(browser_name, 16, "Browser name")?;
        let (platform, arch, release) = bounded_server_connect_runtime_identity(runtime_identity)?;
        let user_agent = if let Some(cached) = DEFAULT_PLAYWRIGHT_USER_AGENT.get() {
            cached.clone()
        } else {
            let inputs = crate::user_agent_runtime::collect(env, platform, arch, release)?;
            DEFAULT_PLAYWRIGHT_USER_AGENT
                .get_or_init(|| format_user_agent(&inputs))
                .clone()
        };
        // The transport currently requires a finite bound. An upstream zero
        // timeout is admitted but conservatively capped at its maximum.
        let budget_ms = if timeout_ms == 0 { 120_000 } else { timeout_ms };
        let discovery_limits = DiscoveryLimits::new(64 * 1024, budget_ms)
            .map_err(|error| binding_error(HostError::InvalidArgument(error.to_string())))?;
        let websocket_limits = WsLimits::new(256 * 1024, 64 * 1024, budget_ms)
            .map_err(|error| binding_error(HostError::InvalidArgument(error.to_string())))?;
        let http_discovery = endpoint
            .get(..7)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
            || endpoint
                .get(..8)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"));
        let prepared_headers = if http_discovery {
            // Validate all syntax, count, duplicate and byte limits now. Only
            // the handshake-reserved-name check waits for HTTP discovery.
            let mut deferred = WsHeaders::default();
            bounded_server_connect_headers_for_http_discovery(headers, &mut deferred)?;
            append_server_connect_defaults_for_http_discovery(
                &mut deferred,
                &browser_name,
                &user_agent,
            )
            .map_err(|error| binding_error(HostError::InvalidArgument(error.to_string())))?;
            deferred
        } else {
            // Direct WS(S) endpoints keep strict preflight before network I/O.
            let placeholder = WsEndpoint::parse("ws://example.invalid/")
                .map_err(|error| binding_error(HostError::Backend(error.to_string())))?;
            let mut request = WsRequest::new(placeholder, websocket_limits);
            bounded_server_connect_headers(headers, &mut request)?;
            append_server_connect_defaults(&mut request, &browser_name, &user_agent)
                .map_err(|error| binding_error(HostError::InvalidArgument(error.to_string())))?;
            request.headers().clone()
        };
        let wall_time_ms: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| {
                binding_error(HostError::Backend(
                    "system clock precedes the Unix epoch".into(),
                ))
            })?
            .as_millis()
            .try_into()
            .map_err(|_| {
                binding_error(HostError::Backend(
                    "system clock milliseconds exceed the supported range".into(),
                ))
            })?;
        let mut options = ConnectOptions::new(
            endpoint,
            user_agent,
            discovery_limits,
            websocket_limits,
            SdkLanguage::Javascript,
            wall_time_ms,
        );
        for header in prepared_headers.iter() {
            let result = if http_discovery {
                options.add_websocket_header_after_http_discovery(header.name(), header.value())
            } else {
                options.add_websocket_header(header.name(), header.value())
            };
            result.map_err(|error| binding_error(HostError::InvalidArgument(error.to_string())))?;
        }
        let admission = self.owner.admit().map_err(binding_error)?;
        let owner = Arc::clone(&self.owner);
        let ffi = self.owner.ffi.clone();
        #[cfg(feature = "playwright-server-connect-page-events")]
        let glob_factory = pwright_node_host::frame_tree::FrameTreeFactory::new(
            pwright_node_host::frame_tree::FrameTreeLimits {
                states: 1,
                frames_per_page: 1,
                revisions: 32,
                bytes: 8 * 1024 * 1024,
            },
        )
        .map_err(binding_error)?;
        #[cfg(feature = "playwright-server-connect-page-events")]
        let page_events_enabled =
            event_policy == pwright_playwright_server_session::EventPolicy::ProjectPageNotices;
        boundary::spawn_with_env(
            env,
            execution,
            async move {
                let supervisor = tokio::select! {
                    biased;
                    () = owner.stopped.cancelled() => return Err(HostError::TargetClosed),
                    result = tokio::time::timeout(
                        std::time::Duration::from_millis(u64::from(budget_ms)),
                        connect_live_with_event_policy(&options, event_policy),
                    ) => Arc::new(result.map_err(|_| HostError::Timeout(budget_ms))?
                        .map_err(|error| map_server_connect_error(error, budget_ms))?),
                };
                if let Err(error) = admission.publish_remote(&supervisor) {
                    let _ = supervisor.close().await;
                    return Err(error);
                }
                Ok(supervisor)
            },
            move |_, supervisor| {
                Ok(crate::remote_browser::NativeRemoteBrowser {
                    supervisor,
                    ffi,
                    #[cfg(feature = "playwright-server-connect-page-events")]
                    glob_factory,
                    #[cfg(feature = "playwright-server-connect-page-events")]
                    page_events_enabled,
                })
            },
        )
    }
}

/// A fresh Browser with one native process lease and no published default
/// Context. The temporary profile remains owned by this environment until
/// the session closes or the environment drains.
#[cfg(feature = "owned-launch")]
#[napi]
impl NativeEnvironment {
    #[napi]
    pub fn launch_chromium<'env>(
        &self,
        env: &'env Env,
        executable_path: JsString<'env>,
        headless: bool,
        timeout_ms: u32,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        self.launch_chromium_impl(
            env,
            executable_path,
            headless,
            timeout_ms,
            #[cfg(feature = "playwright-trace")]
            None,
        )
    }

    /// Explicit caller directory capability. The legacy method never accepts
    /// or infers this path; no Context recording is installed by this port.
    #[cfg(feature = "playwright-trace")]
    #[napi]
    pub fn launch_chromium_with_traces_dir<'env>(
        &self,
        env: &'env Env,
        executable_path: JsString<'env>,
        headless: bool,
        timeout_ms: u32,
        traces_dir: JsString<'env>,
        base_directory: JsString<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let traces_dir = boundary::bounded_string(traces_dir, 4096, "tracesDir")?;
        let base_directory =
            boundary::bounded_string(base_directory, 4096, "trace base directory")?;
        self.launch_chromium_impl(
            env,
            executable_path,
            headless,
            timeout_ms,
            Some((base_directory, traces_dir)),
        )
    }
}

#[cfg(feature = "owned-launch")]
impl NativeEnvironment {
    fn launch_chromium_impl<'env>(
        &self,
        env: &'env Env,
        executable_path: JsString<'env>,
        headless: bool,
        timeout_ms: u32,
        #[cfg(feature = "playwright-trace")] trace_directory: Option<(String, String)>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .owner
            .ffi
            .admit(FfiLane::Connect, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let executable_path = boundary::bounded_string(executable_path, 8192, "executablePath")?;
        let admission = self.owner.admit().map_err(binding_error)?;
        let owner = Arc::clone(&self.owner);
        let ffi = self.owner.ffi.clone();
        #[cfg(feature = "handles")]
        let wait_admissions = self.owner.wait_admissions.clone();
        let queries = self.owner.queries.clone();
        #[cfg(feature = "selector-registration")]
        let selectors = self.owner.selectors.clone();
        let conversions = self.conversions.clone();
        let locator_conversions = self.locator_conversions.clone();
        let publications = self.publications.clone();
        #[cfg(feature = "retained-file-input")]
        let file_input_timeout_budget = self.file_input_timeouts.clone();
        #[cfg(feature = "retained-file-input")]
        let file_input_preparations = self.file_input_preparations.clone();
        boundary::spawn_with_env(
            env,
            execution,
            async move {
                #[cfg(feature = "playwright-trace")]
                let trace_directory = if let Some((base, requested)) = trace_directory
                    && !requested.is_empty()
                {
                    Some(
                        prepare_trace_directory(
                            Path::new(&base),
                            Path::new(&requested),
                            TraceDirectoryLimits::new(4096, 256),
                        )
                        .map_err(|error| {
                            HostError::InvalidArgument(format!("tracesDir: {error}"))
                        })?,
                    )
                } else {
                    None
                };
                let executable = ChromiumExecutable::from_path(
                    PathBuf::from(executable_path),
                    ChromiumChannel::Custom,
                )
                .map_err(|error| HostError::InvalidArgument(format!("executablePath: {error}")))?;
                let directory = tempfile::Builder::new()
                    .prefix("pwright-node-")
                    .tempdir()
                    .map_err(|error| {
                        HostError::Backend(format!("create Chromium profile: {error}"))
                    })?;
                let profile_path = std::fs::canonicalize(directory.path()).map_err(|error| {
                    HostError::Backend(format!("resolve Chromium profile: {error}"))
                })?;
                let profile = Arc::new(Mutex::new(Some(directory)));
                let host = tokio::select! {
                    biased;
                    () = owner.stopped.cancelled() => return Err(HostError::TargetClosed),
                    result = async {
                        #[cfg(feature = "playwright-trace")]
                        if let Some(trace_directory) = trace_directory {
                            return HostSession::launch_chromium_with_trace_directory(
                                executable,
                                profile_path,
                                headless,
                                timeout_ms,
                                trace_directory,
                            ).await;
                        }
                        HostSession::launch_chromium_with_lifecycle_events(
                            executable, profile_path, headless, timeout_ms,
                        ).await
                    } => Arc::new(result?),
                };
                admission.publish(&host, Some(Arc::clone(&profile)))?;
                Ok((host, profile))
            },
            move |env, (host, profile)| {
                #[cfg(feature = "retained-file-input")]
                let file_input_scope = match file_input_preparations.scope() {
                    Ok(scope) => scope,
                    Err(error) => {
                        host.frame_publications().discard();
                        host.request_shutdown();
                        return Err(binding_error(error));
                    }
                };
                let port = (|| {
                    let port = publications.admit(&host).map_err(binding_error)?;
                    #[cfg(feature = "retained-file-input")]
                    port.bind_file_input_scope(file_input_scope.clone());
                    port.publish(
                        env,
                        host.frame_publications().fence().map_err(binding_error)?,
                    )?;
                    Ok::<_, napi::Error>(port)
                })();
                let port = match port {
                    Ok(port) => port,
                    Err(error) => {
                        host.frame_publications().discard();
                        host.request_shutdown();
                        return Err(error);
                    }
                };
                Ok(NativeSession {
                    launch_profile: Some(profile),
                    #[cfg(feature = "retained-file-input")]
                    file_input_timeout_budget,
                    #[cfg(feature = "retained-file-input")]
                    file_input_scope,
                    #[cfg(feature = "retained-file-input")]
                    file_input_preparations,
                    publications: port,
                    host,
                    ffi,
                    #[cfg(feature = "handles")]
                    wait_admissions,
                    conversions,
                    locator_conversions,
                    queries,
                    #[cfg(feature = "selector-registration")]
                    selectors,
                })
            },
        )
    }
}

/// This opt-in port proves only the error path for a failed, explicitly owned
/// browser process under BiDi test configuration. It cannot return a Browser.
#[cfg(feature = "bidi-startup-failure")]
#[napi]
impl NativeEnvironment {
    #[napi(js_name = "observeBidiStartupFailure")]
    pub fn observe_bidi_startup_failure<'env>(
        &self,
        env: &'env Env,
        engine: JsString<'env>,
        executable_path: JsString<'env>,
        timeout_ms: u32,
        traces_dir: JsString<'env>,
        base_directory: JsString<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .owner
            .ffi
            .admit(FfiLane::Connect, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let engine = boundary::bounded_string(engine, 16, "engine")?;
        let engine = StartupEngine::parse(&engine).map_err(binding_error)?;
        let executable_path = boundary::bounded_string(executable_path, 8192, "executablePath")?;
        let traces_dir = boundary::bounded_string(traces_dir, 4096, "tracesDir")?;
        let base_directory =
            boundary::bounded_string(base_directory, 4096, "trace base directory")?;
        let admission = self.owner.admit().map_err(binding_error)?;
        let owner = Arc::clone(&self.owner);
        boundary::spawn_with_env(
            env,
            execution,
            async move {
                // Keep the finite pending admission and its teardown barrier
                // until the owned process has been observed and reaped.
                let _admission = admission;
                if !traces_dir.is_empty() {
                    prepare_trace_directory(
                        Path::new(&base_directory),
                        Path::new(&traces_dir),
                        TraceDirectoryLimits::new(4096, 256),
                    )
                    .map_err(|error| HostError::InvalidArgument(format!("tracesDir: {error}")))?;
                }
                let executable = validate_startup_executable(PathBuf::from(executable_path))?;
                let profile = tempfile::Builder::new()
                    .prefix("pwright-node-bidi-startup-")
                    .tempdir()
                    .map_err(|error| {
                        HostError::Backend(format!("create browser profile: {error}"))
                    })?;
                let profile_path = std::fs::canonicalize(profile.path()).map_err(|error| {
                    HostError::Backend(format!("resolve browser profile: {error}"))
                })?;
                let arguments = engine.arguments(&profile_path)?;
                let report = tokio::select! {
                    biased;
                    () = owner.stopped.cancelled() => return Err(HostError::TargetClosed),
                    result = observe_explicit_startup(
                        &executable,
                        &arguments,
                        std::time::Duration::from_millis(u64::from(timeout_ms)),
                    ) => result.map_err(|error| HostError::Backend(format!("observe browser startup: {error}")))?,
                };
                Err::<bool, HostError>(startup_report_error(report))
            },
            |_, result| Ok(result),
        )
    }
}

#[cfg(feature = "bidi-startup-failure")]
#[derive(Clone, Copy)]
enum StartupEngine {
    Chromium,
    Firefox,
}

#[cfg(feature = "bidi-startup-failure")]
impl StartupEngine {
    fn parse(value: &str) -> Result<Self, HostError> {
        match value {
            "chromium" => Ok(Self::Chromium),
            "firefox" => Ok(Self::Firefox),
            _ => Err(HostError::InvalidArgument(
                "engine must be chromium or firefox".into(),
            )),
        }
    }

    fn arguments(self, profile_path: &Path) -> Result<Vec<String>, HostError> {
        let profile_path = profile_path
            .to_str()
            .ok_or_else(|| HostError::Backend("browser profile path is not valid UTF-8".into()))?;
        Ok(match self {
            Self::Chromium => vec![format!("--user-data-dir={profile_path}")],
            Self::Firefox => vec!["--profile".into(), profile_path.to_owned()],
        })
    }
}

#[cfg(feature = "bidi-startup-failure")]
fn validate_startup_executable(requested: PathBuf) -> Result<PathBuf, HostError> {
    if !requested.is_absolute() {
        return Err(HostError::InvalidArgument(
            "executablePath must be absolute".into(),
        ));
    }
    let executable = std::fs::canonicalize(&requested).map_err(|error| {
        HostError::InvalidArgument(format!("executablePath: cannot resolve file: {error}"))
    })?;
    let metadata = std::fs::metadata(&executable).map_err(|error| {
        HostError::InvalidArgument(format!("executablePath: cannot inspect file: {error}"))
    })?;
    if !metadata.is_file() {
        return Err(HostError::InvalidArgument(
            "executablePath is not a regular file".into(),
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(HostError::InvalidArgument(
                "executablePath is not executable".into(),
            ));
        }
    }
    Ok(executable)
}

#[cfg(feature = "bidi-startup-failure")]
fn startup_report_error(report: StartupReport) -> HostError {
    match report.outcome {
        StartupObservation::ExitedBeforeReady(status) if !status.success() => {
            let stdout = String::from_utf8_lossy(&report.stdout);
            let stderr = String::from_utf8_lossy(&report.stderr);
            HostError::Backend(format!(
                "Failed to launch the browser process.\nBrowser logs:\n<process exited: {status}>\n<stdout>\n{stdout}\n<stderr>\n{stderr}"
            ))
        }
        StartupObservation::ExitedBeforeReady(_) | StartupObservation::RunningWithoutProtocol => {
            HostError::UnsupportedOperation(
                "BiDi browser startup without a nonzero early exit is not supported".into(),
            )
        }
    }
}
