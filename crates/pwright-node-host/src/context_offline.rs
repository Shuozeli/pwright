//! Optional Context-scoped Chromium offline policy.
//!
//! Rust retains a fixed Context registry and applies Playwright's Chromium
//! network-emulation payload to every newly initialized Page or out-of-process
//! Frame session before it is resumed or published.

use std::error::Error;
use std::future::Future;
use std::sync::{Arc, Mutex};

use pwright_backend_cdp_topology::CdpContextOwnership;
use pwright_cdp::connection::CdpError;
use pwright_cdp::{CdpConnection, CdpSession};
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequirements,
    ResourceScope,
};
use pwright_plugin_chromium_session_initialization::{
    ChromiumSessionInitializationFuture, ChromiumSessionInitializationTarget,
    ChromiumSessionInitializer,
};

use crate::{HostError, HostSession};

pub(crate) const MAX_CONTEXTS: usize = 16;
const MAX_SESSIONS: usize = 80; // 16 Pages and at most 64 Frame sessions.
const SCOPE: ResourceScope = ResourceScope::extension(193, 0);

struct State {
    default_offline: bool,
    default_dirty: bool,
    contexts: Vec<ObjectId>,
    dirty_contexts: Vec<ObjectId>,
    closed: bool,
}

#[derive(Clone, Copy)]
struct ChangeState {
    previous: bool,
    dirty: bool,
}

struct NativeSession {
    session_id: String,
    target_id: String,
    is_page: bool,
}

pub(crate) const fn requirements() -> ResourceRequirements {
    let storage_bytes = (std::mem::size_of::<Installation>()
        + std::mem::size_of::<State>()
        + 2 * MAX_CONTEXTS * std::mem::size_of::<ObjectId>()) as u64;
    ResourceRequirements::none()
        .with_class(
            ResourceClass::BackendStateBytes,
            storage_bytes,
            storage_bytes,
        )
        .with_metadata(1, 1)
}

pub(crate) struct Installation {
    state: Mutex<State>,
    gate: tokio::sync::Mutex<()>,
    _storage: ResourceLease,
}

impl Installation {
    pub(crate) fn new_with_default(
        resources: &ProductionResourceLedger,
        default_offline: bool,
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
        let mut contexts = Vec::new();
        contexts
            .try_reserve_exact(MAX_CONTEXTS)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        let mut dirty_contexts = Vec::new();
        dirty_contexts
            .try_reserve_exact(MAX_CONTEXTS)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        Ok(Arc::new(Self {
            state: Mutex::new(State {
                default_offline,
                default_dirty: false,
                contexts,
                dirty_contexts,
                closed: false,
            }),
            gate: tokio::sync::Mutex::new(()),
            _storage: storage,
        }))
    }

    pub(crate) fn install(&self, context: ObjectId) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        if state.contexts.contains(&context) {
            return Err(HostError::InvalidArgument(
                "offline policy is already installed for this Context".into(),
            ));
        }
        if state.contexts.len() + usize::from(state.default_offline) == MAX_CONTEXTS {
            return Err(HostError::ResourceExhausted(format!(
                "offline policy exceeds {MAX_CONTEXTS} live Contexts"
            )));
        }
        state.contexts.push(context);
        Ok(())
    }

    pub(crate) fn remove(&self, context: ObjectId) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        state.contexts.retain(|candidate| *candidate != context);
        state
            .dirty_contexts
            .retain(|candidate| *candidate != context);
        Ok(())
    }

    pub(crate) async fn remove_serialized(&self, context: ObjectId) -> Result<(), HostError> {
        let _gate = self.gate.lock().await;
        self.remove(context)
    }

    fn previous_for_change(
        state: &State,
        context: ObjectId,
        ownership: CdpContextOwnership,
        offline: bool,
    ) -> Result<bool, HostError> {
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        match ownership {
            CdpContextOwnership::Default => {
                if offline && !state.default_offline && state.contexts.len() == MAX_CONTEXTS {
                    return Err(HostError::ResourceExhausted(format!(
                        "offline policy exceeds {MAX_CONTEXTS} live Contexts"
                    )));
                }
                Ok(state.default_offline)
            }
            CdpContextOwnership::Owned => {
                let previous = state.contexts.contains(&context);
                if offline
                    && !previous
                    && state.contexts.len() + usize::from(state.default_offline) == MAX_CONTEXTS
                {
                    return Err(HostError::ResourceExhausted(format!(
                        "offline policy exceeds {MAX_CONTEXTS} live Contexts"
                    )));
                }
                Ok(previous)
            }
            CdpContextOwnership::External => Err(HostError::UnsupportedOperation(
                "offline policy cannot be changed for an externally owned Context".into(),
            )),
        }
    }

    fn prepare_change(
        &self,
        context: ObjectId,
        ownership: CdpContextOwnership,
        offline: bool,
    ) -> Result<ChangeState, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        let previous = Self::previous_for_change(&state, context, ownership, offline)?;
        let dirty = match ownership {
            CdpContextOwnership::Default => state.default_dirty,
            CdpContextOwnership::Owned => state.dirty_contexts.contains(&context),
            CdpContextOwnership::External => unreachable!("checked by previous_for_change"),
        };
        Ok(ChangeState { previous, dirty })
    }

    fn mark_dirty(
        &self,
        context: ObjectId,
        ownership: CdpContextOwnership,
        offline: bool,
    ) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        Self::previous_for_change(&state, context, ownership, offline)?;
        match ownership {
            CdpContextOwnership::Default if !state.default_dirty => {
                if state.dirty_contexts.len() == MAX_CONTEXTS {
                    return Err(HostError::ResourceExhausted(format!(
                        "offline reconciliation exceeds {MAX_CONTEXTS} live Contexts"
                    )));
                }
                state.default_dirty = true;
            }
            CdpContextOwnership::Owned if !state.dirty_contexts.contains(&context) => {
                if state.dirty_contexts.len() + usize::from(state.default_dirty) == MAX_CONTEXTS {
                    return Err(HostError::ResourceExhausted(format!(
                        "offline reconciliation exceeds {MAX_CONTEXTS} live Contexts"
                    )));
                }
                state.dirty_contexts.push(context);
            }
            CdpContextOwnership::Default | CdpContextOwnership::Owned => {}
            CdpContextOwnership::External => unreachable!("checked by previous_for_change"),
        }
        Ok(())
    }

    fn set(
        &self,
        context: ObjectId,
        ownership: CdpContextOwnership,
        offline: bool,
    ) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        Self::previous_for_change(&state, context, ownership, offline)?;
        match ownership {
            CdpContextOwnership::Default => {
                state.default_offline = offline;
                state.default_dirty = false;
            }
            CdpContextOwnership::Owned => {
                if offline {
                    if !state.contexts.contains(&context) {
                        state.contexts.push(context);
                    }
                } else {
                    state.contexts.retain(|candidate| *candidate != context);
                }
                state
                    .dirty_contexts
                    .retain(|candidate| *candidate != context);
            }
            CdpContextOwnership::External => unreachable!("checked by previous_for_change"),
        }
        Ok(())
    }

    fn offline(
        &self,
        context: ObjectId,
        ownership: CdpContextOwnership,
    ) -> Result<bool, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        Ok(match ownership {
            CdpContextOwnership::Default => state.default_offline,
            CdpContextOwnership::Owned => state.contexts.contains(&context),
            CdpContextOwnership::External => false,
        })
    }

    fn close(&self) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.closed = true;
        state.contexts.clear();
        state.dirty_contexts.clear();
        state.default_dirty = false;
    }
}

impl Drop for Installation {
    fn drop(&mut self) {
        self.close();
    }
}

fn offline_payload(offline: bool) -> serde_json::Value {
    serde_json::json!({
        "offline": offline,
        "latency": 0,
        "downloadThroughput": -1,
        "uploadThroughput": -1
    })
}

fn retired_frame_session(session: &NativeSession, error: &CdpError) -> bool {
    !session.is_page
        && (matches!(error, CdpError::SessionDetached)
            || matches!(error, CdpError::Protocol { code: -32001, message }
                if message == "Session with given id not found."))
}

async fn restore_sessions<F, Fut>(
    sessions: &[NativeSession],
    attempted: &[usize],
    previous: bool,
    send: &mut F,
) -> Result<(), HostError>
where
    F: FnMut(usize, bool) -> Fut,
    Fut: Future<Output = Result<(), CdpError>>,
{
    let mut failures = 0;
    let mut first = None;
    for &index in attempted.iter().rev() {
        if let Err(error) = send(index, previous).await {
            if retired_frame_session(&sessions[index], &error) {
                continue;
            }
            failures += 1;
            if first.is_none() {
                first = Some(error);
            }
        }
    }
    if let Some(error) = first {
        Err(HostError::backend(format_args!(
            "offline rollback failed for {failures} native sessions; first error: {error}"
        )))
    } else {
        Ok(())
    }
}

async fn failed_change_with_rollback<F, Fut>(
    sessions: &[NativeSession],
    attempted: &[usize],
    previous: bool,
    primary: HostError,
    send: &mut F,
) -> Result<(), HostError>
where
    F: FnMut(usize, bool) -> Fut,
    Fut: Future<Output = Result<(), CdpError>>,
{
    match restore_sessions(sessions, attempted, previous, send).await {
        // Even a complete compensation of this attempt cannot prove that an
        // older dirty Context's unvisited sessions match the retained policy.
        // Only a full successful broadcast can clear the dirty marker.
        Ok(()) => Err(primary),
        Err(rollback) => Err(HostError::backend(format_args!(
            "offline update failed: {primary}; {rollback}"
        ))),
    }
}

async fn broadcast_and_commit<F, Fut>(
    installation: &Installation,
    context: ObjectId,
    ownership: CdpContextOwnership,
    offline: bool,
    sessions: &[NativeSession],
    mut send: F,
) -> Result<(), HostError>
where
    F: FnMut(usize, bool) -> Fut,
    Fut: Future<Output = Result<(), CdpError>>,
{
    let change = installation.prepare_change(context, ownership, offline)?;
    if change.previous == offline && !change.dirty {
        return Ok(());
    }
    let mut attempted = Vec::new();
    attempted
        .try_reserve_exact(sessions.len())
        .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
    // A dropped async call can leave an in-flight CDP command applied. Mark
    // the Context before the first send so a later same-value call reconciles.
    installation.mark_dirty(context, ownership, offline)?;
    for index in 0..sessions.len() {
        match send(index, offline).await {
            Ok(()) => attempted.push(index),
            Err(error) if retired_frame_session(&sessions[index], &error) => {}
            Err(error) => {
                // A timeout or dropped response does not prove that the
                // command had no effect; compensate the failed index too.
                attempted.push(index);
                let primary = HostError::backend(error);
                return failed_change_with_rollback(
                    sessions,
                    &attempted,
                    change.previous,
                    primary,
                    &mut send,
                )
                .await;
            }
        }
    }
    if let Err(primary) = installation.set(context, ownership, offline) {
        return failed_change_with_rollback(
            sessions,
            &attempted,
            change.previous,
            primary,
            &mut send,
        )
        .await;
    }
    Ok(())
}

pub(crate) struct Initializer<Downstream> {
    downstream: Downstream,
    installation: Arc<Installation>,
    connection: Arc<CdpConnection>,
}

impl<Downstream> Initializer<Downstream> {
    pub(crate) fn new(
        downstream: Downstream,
        installation: Arc<Installation>,
        connection: Arc<CdpConnection>,
    ) -> Self {
        Self {
            downstream,
            installation,
            connection,
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
            self.downstream.initialize_scoped(target).await?;
            let _gate = self.installation.gate.lock().await;
            if !self
                .installation
                .offline(target.context, target.ownership)?
            {
                return Ok(());
            }
            CdpSession::new(
                Arc::clone(&self.connection),
                target.session_id.to_owned(),
                target.target_id.to_owned(),
            )
            .send("Network.emulateNetworkConditions", offline_payload(true))
            .await?;
            Ok::<(), Box<dyn Error + Send + Sync>>(())
        })
    }
}

impl HostSession {
    /// Apply a Context's new network state to current Page/OOPIF sessions and
    /// retain it for sessions created after this operation completes.
    pub async fn set_context_offline(
        &self,
        context_id: &str,
        offline: bool,
    ) -> Result<(), HostError> {
        let (context, _call, _operation) = self.admit_operation(context_id)?;
        let installation = {
            let state = self.state.lock().map_err(HostError::backend)?;
            state
                .as_ref()
                .ok_or(HostError::TargetClosed)?
                .context_offline
                .clone()
        };
        let _gate = tokio::select! {
            biased;
            () = self.stop.cancelled() => return Err(HostError::TargetClosed),
            gate = installation.gate.lock() => gate,
        };
        let (ownership, connection, sessions) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            let topology = topology.topology();
            let route = topology
                .browser_context_route(context)
                .ok_or(HostError::TargetClosed)?;
            let change = installation.prepare_change(context, route.ownership(), offline)?;
            if change.previous == offline && !change.dirty {
                return Ok(());
            }
            let page_targets = topology
                .page_session_routes()
                .filter(|page| page.browser_context_object_id() == context)
                .map(|page| page.target_id())
                .collect::<Vec<_>>();
            let mut sessions = Vec::new();
            sessions
                .try_reserve_exact(MAX_SESSIONS)
                .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
            for target in topology.target_session_routes() {
                if !page_targets.contains(&target.page_target_id()) {
                    continue;
                }
                if sessions.len() == MAX_SESSIONS {
                    return Err(HostError::ResourceExhausted(
                        "offline native session capacity exceeded".into(),
                    ));
                }
                sessions.push(NativeSession {
                    session_id: target.session_id().to_owned(),
                    target_id: target.target_id().to_owned(),
                    is_page: target.target_id() == target.page_target_id(),
                });
            }
            (
                route.ownership(),
                live.composition.connection().clone(),
                sessions,
            )
        };
        broadcast_and_commit(
            &installation,
            context,
            ownership,
            offline,
            &sessions,
            |index, value| {
                let session = CdpSession::new(
                    connection.clone(),
                    sessions[index].session_id.clone(),
                    sessions[index].target_id.clone(),
                );
                async move {
                    session
                        .send("Network.emulateNetworkConditions", offline_payload(value))
                        .await
                        .map(|_| ())
                }
            },
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_core::ResourceLimits;
    use std::sync::atomic::{AtomicBool, Ordering};

    fn native_sessions(count: usize) -> Vec<NativeSession> {
        (0..count)
            .map(|index| NativeSession {
                session_id: format!("session-{index}"),
                target_id: format!("page-{index}"),
                is_page: true,
            })
            .collect()
    }

    fn resources() -> ProductionResourceLedger {
        let required = requirements();
        let mut limits = ResourceLimits::deny_all(required.scopes, required.leases);
        for class in ResourceClass::ALL {
            let limit = required.class(class);
            limits = limits.with_class(class, limit.global, limit.per_scope);
        }
        ProductionResourceLedger::new(limits, required).expect("valid offline policy plan")
    }

    #[test]
    fn policy_is_context_scoped_and_release_is_idempotent() {
        // Arrange
        let installation = Installation::new_with_default(&resources(), false).unwrap();
        let context = ObjectId::new(7);

        // Act
        installation.install(context).unwrap();
        let installed = installation
            .offline(context, CdpContextOwnership::Owned)
            .unwrap();
        installation.remove(context).unwrap();
        installation.remove(context).unwrap();

        // Assert
        assert!(installed);
        assert!(
            !installation
                .offline(context, CdpContextOwnership::Owned)
                .unwrap()
        );
    }

    #[test]
    fn policy_rejects_more_than_the_fixed_context_capacity() {
        // Arrange
        let installation = Installation::new_with_default(&resources(), false).unwrap();
        for value in 1..=MAX_CONTEXTS {
            installation.install(ObjectId::new(value as u64)).unwrap();
        }

        // Act
        let error = installation
            .install(ObjectId::new((MAX_CONTEXTS + 1) as u64))
            .unwrap_err();

        // Assert
        assert!(matches!(error, HostError::ResourceExhausted(_)));
    }

    #[test]
    fn default_policy_is_selected_only_for_the_default_context() {
        // Arrange
        let installation = Installation::new_with_default(&resources(), true).unwrap();
        let context = ObjectId::new(7);

        // Act
        let default_offline = installation
            .offline(context, CdpContextOwnership::Default)
            .unwrap();
        let owned_offline = installation
            .offline(context, CdpContextOwnership::Owned)
            .unwrap();
        let external_offline = installation
            .offline(context, CdpContextOwnership::External)
            .unwrap();

        // Assert
        assert!(default_offline);
        assert!(!owned_offline);
        assert!(!external_offline);
    }

    #[test]
    fn default_policy_consumes_one_fixed_context_slot() {
        // Arrange
        let installation = Installation::new_with_default(&resources(), true).unwrap();
        for value in 1..MAX_CONTEXTS {
            installation.install(ObjectId::new(value as u64)).unwrap();
        }

        // Act
        let error = installation
            .install(ObjectId::new(MAX_CONTEXTS as u64))
            .unwrap_err();

        // Assert
        assert!(matches!(error, HostError::ResourceExhausted(_)));
    }

    #[test]
    fn protocol_payload_matches_playwright_chromium() {
        // Arrange
        let expected = serde_json::json!({
            "offline": true,
            "latency": 0,
            "downloadThroughput": -1,
            "uploadThroughput": -1
        });

        // Act
        let payload = offline_payload(true);

        // Assert
        assert_eq!(payload, expected);
    }

    #[test]
    fn policy_can_toggle_an_existing_owned_context() {
        // Arrange
        let installation = Installation::new_with_default(&resources(), false).unwrap();
        let context = ObjectId::new(7);

        // Act
        installation
            .set(context, CdpContextOwnership::Owned, true)
            .unwrap();
        let offline = installation
            .offline(context, CdpContextOwnership::Owned)
            .unwrap();
        installation
            .set(context, CdpContextOwnership::Owned, false)
            .unwrap();

        // Assert
        assert!(offline);
        assert!(
            !installation
                .offline(context, CdpContextOwnership::Owned)
                .unwrap()
        );
        assert_eq!(offline_payload(false)["offline"], false);
    }

    #[test]
    fn policy_can_restore_an_initially_offline_default_context() {
        // Arrange
        let installation = Installation::new_with_default(&resources(), true).unwrap();
        let context = ObjectId::new(7);

        // Act
        installation
            .set(context, CdpContextOwnership::Default, false)
            .unwrap();
        let online = installation
            .offline(context, CdpContextOwnership::Default)
            .unwrap();
        installation
            .set(context, CdpContextOwnership::Default, true)
            .unwrap();

        // Assert
        assert!(!online);
        assert!(
            installation
                .offline(context, CdpContextOwnership::Default)
                .unwrap()
        );
    }

    #[tokio::test]
    async fn failed_later_session_restores_completed_sessions_and_keeps_policy() {
        // Arrange
        let installation = Installation::new_with_default(&resources(), false).unwrap();
        let context = ObjectId::new(7);
        let sessions = native_sessions(3);
        let calls = Arc::new(Mutex::new(Vec::new()));

        // Act
        let result = broadcast_and_commit(
            &installation,
            context,
            CdpContextOwnership::Owned,
            true,
            &sessions,
            |index, offline| {
                let calls = Arc::clone(&calls);
                async move {
                    calls.lock().unwrap().push((index, offline));
                    if index == 2 && offline {
                        Err(CdpError::Other("injected third-session failure".into()))
                    } else {
                        Ok(())
                    }
                }
            },
        )
        .await;

        // Assert
        assert!(matches!(result, Err(HostError::Backend(_))));
        assert_eq!(
            *calls.lock().unwrap(),
            [
                (0, true),
                (1, true),
                (2, true),
                (2, false),
                (1, false),
                (0, false),
            ]
        );
        assert!(
            installation
                .prepare_change(context, CdpContextOwnership::Owned, false)
                .unwrap()
                .dirty
        );
        assert!(
            !installation
                .offline(context, CdpContextOwnership::Owned)
                .unwrap()
        );
    }

    #[tokio::test]
    async fn rollback_failure_is_reported_after_attempting_other_compensations() {
        // Arrange
        let installation = Installation::new_with_default(&resources(), false).unwrap();
        let context = ObjectId::new(7);
        let sessions = native_sessions(3);
        let calls = Arc::new(Mutex::new(Vec::new()));

        // Act
        let result = broadcast_and_commit(
            &installation,
            context,
            CdpContextOwnership::Owned,
            true,
            &sessions,
            |index, offline| {
                let calls = Arc::clone(&calls);
                async move {
                    calls.lock().unwrap().push((index, offline));
                    if (index == 2 && offline) || (index == 1 && !offline) {
                        Err(CdpError::Other("injected CDP failure".into()))
                    } else {
                        Ok(())
                    }
                }
            },
        )
        .await;

        // Assert
        assert!(
            matches!(result, Err(HostError::Backend(message)) if message.contains("rollback failed for 1 native sessions"))
        );
        assert_eq!(
            *calls.lock().unwrap(),
            [
                (0, true),
                (1, true),
                (2, true),
                (2, false),
                (1, false),
                (0, false),
            ]
        );
        assert!(
            installation
                .prepare_change(context, CdpContextOwnership::Owned, false)
                .unwrap()
                .dirty
        );
        assert!(
            !installation
                .offline(context, CdpContextOwnership::Owned)
                .unwrap()
        );
    }

    #[tokio::test]
    async fn same_value_reconciles_timeout_after_apply_for_owned_and_default_contexts() {
        for ownership in [CdpContextOwnership::Owned, CdpContextOwnership::Default] {
            // Arrange
            let installation = Installation::new_with_default(&resources(), false).unwrap();
            let context = ObjectId::new(7);
            let sessions = native_sessions(2);
            let native = Arc::new(Mutex::new([false, false]));
            let fail_first_rollback = Arc::new(AtomicBool::new(true));

            // Act
            let first = broadcast_and_commit(
                &installation,
                context,
                ownership,
                true,
                &sessions,
                |index, offline| {
                    let native = Arc::clone(&native);
                    let fail_first_rollback = Arc::clone(&fail_first_rollback);
                    async move {
                        if index == 1
                            && !offline
                            && fail_first_rollback.swap(false, Ordering::SeqCst)
                        {
                            return Err(CdpError::Timeout);
                        }
                        native.lock().unwrap()[index] = offline;
                        if index == 1 && offline {
                            Err(CdpError::Timeout)
                        } else {
                            Ok(())
                        }
                    }
                },
            )
            .await;
            let dirty_after_failure = installation
                .prepare_change(context, ownership, false)
                .unwrap()
                .dirty;
            let native_after_failure = *native.lock().unwrap();
            let calls = Arc::new(Mutex::new(Vec::new()));
            let reconciled = broadcast_and_commit(
                &installation,
                context,
                ownership,
                false,
                &sessions,
                |index, offline| {
                    let native = Arc::clone(&native);
                    let calls = Arc::clone(&calls);
                    async move {
                        calls.lock().unwrap().push((index, offline));
                        native.lock().unwrap()[index] = offline;
                        Ok(())
                    }
                },
            )
            .await;

            // Assert
            assert!(matches!(first, Err(HostError::Backend(_))));
            assert!(dirty_after_failure);
            assert_eq!(native_after_failure, [false, true]);
            assert!(reconciled.is_ok());
            assert_eq!(*calls.lock().unwrap(), [(0, false), (1, false)]);
            assert_eq!(*native.lock().unwrap(), [false, false]);
            let change = installation
                .prepare_change(context, ownership, false)
                .unwrap();
            assert!(!change.previous);
            assert!(!change.dirty);
        }
    }

    #[tokio::test]
    async fn cancelled_broadcast_keeps_dirty_until_same_value_reconciliation() {
        // Arrange
        let installation = Installation::new_with_default(&resources(), false).unwrap();
        let context = ObjectId::new(7);
        let native = Arc::new(Mutex::new([false, false]));
        let (blocked_tx, blocked_rx) = tokio::sync::oneshot::channel();
        let (_release_tx, release_rx) = tokio::sync::oneshot::channel::<()>();
        let pending_installation = Arc::clone(&installation);
        let pending_native = Arc::clone(&native);

        // Act
        let pending = tokio::spawn(async move {
            let mut blocked_tx = Some(blocked_tx);
            let mut release_rx = Some(release_rx);
            broadcast_and_commit(
                &pending_installation,
                context,
                CdpContextOwnership::Owned,
                true,
                &native_sessions(2),
                |index, offline| {
                    let blocked_tx = if index == 1 { blocked_tx.take() } else { None };
                    let release_rx = if index == 1 { release_rx.take() } else { None };
                    let native = Arc::clone(&pending_native);
                    async move {
                        if let (Some(blocked_tx), Some(release_rx)) = (blocked_tx, release_rx) {
                            blocked_tx.send(()).unwrap();
                            let _ = release_rx.await;
                        }
                        native.lock().unwrap()[index] = offline;
                        Ok(())
                    }
                },
            )
            .await
        });
        blocked_rx.await.unwrap();
        pending.abort();
        assert!(pending.await.unwrap_err().is_cancelled());
        let dirty_after_cancellation = installation
            .prepare_change(context, CdpContextOwnership::Owned, false)
            .unwrap()
            .dirty;
        let native_after_cancellation = *native.lock().unwrap();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let reconciled = broadcast_and_commit(
            &installation,
            context,
            CdpContextOwnership::Owned,
            false,
            &native_sessions(2),
            |index, offline| {
                let native = Arc::clone(&native);
                let calls = Arc::clone(&calls);
                async move {
                    calls.lock().unwrap().push((index, offline));
                    native.lock().unwrap()[index] = offline;
                    Ok(())
                }
            },
        )
        .await;

        // Assert
        assert!(dirty_after_cancellation);
        assert_eq!(native_after_cancellation, [true, false]);
        assert!(reconciled.is_ok());
        assert_eq!(*calls.lock().unwrap(), [(0, false), (1, false)]);
        assert_eq!(*native.lock().unwrap(), [false, false]);
        assert!(
            !installation
                .prepare_change(context, CdpContextOwnership::Owned, false)
                .unwrap()
                .dirty
        );
    }

    #[test]
    fn dirty_registry_is_bounded_and_context_close_reclaims_a_slot() {
        // Arrange
        let installation = Installation::new_with_default(&resources(), false).unwrap();
        installation
            .mark_dirty(ObjectId::new(1), CdpContextOwnership::Default, false)
            .unwrap();
        for value in 2..=MAX_CONTEXTS {
            installation
                .mark_dirty(
                    ObjectId::new(value as u64),
                    CdpContextOwnership::Owned,
                    false,
                )
                .unwrap();
        }

        // Act
        let overflow = installation.mark_dirty(
            ObjectId::new((MAX_CONTEXTS + 1) as u64),
            CdpContextOwnership::Owned,
            false,
        );
        installation.remove(ObjectId::new(2)).unwrap();
        let after_close = installation.mark_dirty(
            ObjectId::new((MAX_CONTEXTS + 1) as u64),
            CdpContextOwnership::Owned,
            false,
        );

        // Assert
        assert!(matches!(overflow, Err(HostError::ResourceExhausted(_))));
        assert!(after_close.is_ok());
        assert!(
            installation
                .prepare_change(
                    ObjectId::new((MAX_CONTEXTS + 1) as u64),
                    CdpContextOwnership::Owned,
                    false,
                )
                .unwrap()
                .dirty
        );
    }

    #[tokio::test]
    async fn unchanged_policy_dispatches_no_native_commands() {
        // Arrange
        let installation = Installation::new_with_default(&resources(), false).unwrap();
        let context = ObjectId::new(7);
        let sessions = native_sessions(2);
        let calls = Arc::new(Mutex::new(Vec::new()));

        // Act
        let result = broadcast_and_commit(
            &installation,
            context,
            CdpContextOwnership::Owned,
            false,
            &sessions,
            |index, offline| {
                let calls = Arc::clone(&calls);
                async move {
                    calls.lock().unwrap().push((index, offline));
                    Ok(())
                }
            },
        )
        .await;

        // Assert
        assert!(result.is_ok());
        assert!(calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn future_session_policy_is_committed_after_broadcast() {
        // Arrange
        let installation = Installation::new_with_default(&resources(), false).unwrap();
        let context = ObjectId::new(7);
        let sessions = native_sessions(2);
        let observed_policy = Arc::new(Mutex::new(Vec::new()));

        // Act
        let result = broadcast_and_commit(
            &installation,
            context,
            CdpContextOwnership::Owned,
            true,
            &sessions,
            |_, _| {
                let observed_policy = Arc::clone(&observed_policy);
                let installation = Arc::clone(&installation);
                async move {
                    observed_policy.lock().unwrap().push(
                        installation
                            .offline(context, CdpContextOwnership::Owned)
                            .unwrap(),
                    );
                    Ok(())
                }
            },
        )
        .await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(*observed_policy.lock().unwrap(), [false, false]);
        assert!(
            installation
                .offline(context, CdpContextOwnership::Owned)
                .unwrap()
        );
    }

    #[tokio::test]
    async fn external_context_rejects_even_unchanged_policy_without_dispatch() {
        // Arrange
        let installation = Installation::new_with_default(&resources(), false).unwrap();
        let context = ObjectId::new(7);
        let sessions = native_sessions(1);
        let calls = Arc::new(Mutex::new(Vec::new()));

        // Act
        let result = broadcast_and_commit(
            &installation,
            context,
            CdpContextOwnership::External,
            false,
            &sessions,
            |index, offline| {
                let calls = Arc::clone(&calls);
                async move {
                    calls.lock().unwrap().push((index, offline));
                    Ok(())
                }
            },
        )
        .await;

        // Assert
        assert!(matches!(result, Err(HostError::UnsupportedOperation(_))));
        assert!(calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn detached_oopif_does_not_fail_other_sessions_or_future_policy() {
        // Arrange
        let installation = Installation::new_with_default(&resources(), false).unwrap();
        let context = ObjectId::new(7);
        let mut sessions = native_sessions(3);
        sessions[1].is_page = false;
        let calls = Arc::new(Mutex::new(Vec::new()));

        // Act
        let result = broadcast_and_commit(
            &installation,
            context,
            CdpContextOwnership::Owned,
            true,
            &sessions,
            |index, offline| {
                let calls = Arc::clone(&calls);
                async move {
                    calls.lock().unwrap().push((index, offline));
                    if index == 1 {
                        Err(CdpError::SessionDetached)
                    } else {
                        Ok(())
                    }
                }
            },
        )
        .await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(*calls.lock().unwrap(), [(0, true), (1, true), (2, true)]);
        assert!(
            installation
                .offline(context, CdpContextOwnership::Owned)
                .unwrap()
        );
    }

    #[tokio::test]
    async fn detached_page_session_remains_a_fatal_broadcast_error() {
        // Arrange
        let installation = Installation::new_with_default(&resources(), false).unwrap();
        let context = ObjectId::new(7);
        let sessions = native_sessions(1);

        // Act
        let result = broadcast_and_commit(
            &installation,
            context,
            CdpContextOwnership::Owned,
            true,
            &sessions,
            |_, _| async { Err(CdpError::SessionDetached) },
        )
        .await;

        // Assert
        assert!(matches!(result, Err(HostError::Backend(_))));
        assert!(
            !installation
                .offline(context, CdpContextOwnership::Owned)
                .unwrap()
        );
    }

    #[tokio::test]
    async fn context_removal_waits_for_the_offline_change_gate() {
        // Arrange
        let installation = Installation::new_with_default(&resources(), false).unwrap();
        let context = ObjectId::new(7);
        installation
            .set(context, CdpContextOwnership::Owned, true)
            .unwrap();
        let gate = installation.gate.lock().await;
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let cleanup_installation = Arc::clone(&installation);

        // Act
        let cleanup = tokio::spawn(async move {
            entered_tx.send(()).unwrap();
            cleanup_installation.remove_serialized(context).await
        });
        entered_rx.await.unwrap();
        let waited = !cleanup.is_finished();
        drop(gate);
        cleanup.await.unwrap().unwrap();

        // Assert
        assert!(waited);
        assert!(
            !installation
                .offline(context, CdpContextOwnership::Owned)
                .unwrap()
        );
    }
}
