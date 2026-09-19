//! Node environment ownership, independent of JS object finalization order.
//! Finite admission covers weak sessions and their unpublished metadata endpoint.

use std::sync::{Arc, Mutex, Weak};

use napi::{
    Env, JsString, Unknown,
    bindgen_prelude::{Function, PromiseRaw},
};
use napi_derive::napi;
use pwright_node_host::ffi_budget::{FfiBudget, FfiLane, FfiLimits, FfiMetrics};
use pwright_node_host::locator_query::{QueryBudget, QueryLimits};
use pwright_node_host::{HostError, HostSession, SessionMetrics};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

use crate::deferred_conversion::{ConversionMetrics, DeferredConversions};
use crate::{NativeSession, binding_error, boundary};

const MAX_SESSIONS: usize = 16;

#[derive(Default)]
struct OwnerState {
    sessions: Vec<SessionRecord>,
    pending: usize,
}

#[derive(Clone)]
struct SessionRecord {
    host: Weak<HostSession>,
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
    queries: QueryBudget,
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
        // Cancel pending connects before taking the final published snapshot.
        // No JavaScript callbacks participate in this barrier.
        self.admissions.close();
        self.admissions.wait().await;
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
        if state.sessions.len() + state.pending >= MAX_SESSIONS {
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
    fn publish(mut self, session: &Arc<HostSession>) -> Result<(), HostError> {
        let mut state =
            self.owner.state.lock().map_err(|_| {
                HostError::Backend("Node environment ownership is unavailable".into())
            })?;
        state.pending -= 1;
        self.pending = false;
        state.sessions.push(SessionRecord {
            host: Arc::downgrade(session),
            publications: session.frame_publications(),
            retired: session.disconnect_signal(),
        });
        if self.owner.stopped.is_cancelled() {
            session.request_shutdown();
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
                commands: 32,
                closes: 16,
                watches: 16,
                reserved_bytes: 64 * 1024 * 1024,
                future_bytes: [1024 * 1024, 128 * 1024, 16 * 1024, 16 * 1024],
            })
            .map_err(binding_error)?,
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

    #[napi(js_name = "connectOverCDP")]
    pub fn connect_over_cdp<'env>(
        &self,
        env: &'env Env,
        endpoint: JsString<'env>,
        timeout_ms: u32,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .owner
            .ffi
            .admit(FfiLane::Connect, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let endpoint = boundary::bounded_string(endpoint, 8192, "CDP endpoint")?;
        let admission = self.owner.admit().map_err(binding_error)?;
        let owner = Arc::clone(&self.owner);
        let ffi = self.owner.ffi.clone();
        let queries = self.owner.queries.clone();
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
                let host = tokio::select! {
                    biased;
                    () = owner.stopped.cancelled() => return Err(HostError::TargetClosed),
                    result = HostSession::connect_over_cdp_with_lifecycle_events(&endpoint, timeout_ms) => Arc::new(result?),
                };
                admission.publish(&host)?;
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
                    #[cfg(feature = "retained-file-input")]
                    file_input_timeout_budget,
                    #[cfg(feature = "retained-file-input")]
                    file_input_scope,
                    #[cfg(feature = "retained-file-input")]
                    file_input_preparations,
                    publications: port,
                    host,
                    ffi,
                    conversions,
                    locator_conversions,
                    queries,
                })
            },
        )
    }
}
