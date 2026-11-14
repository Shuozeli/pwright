//! Optional Context-scoped JavaScript disabling policy.
//!
//! Rust retains a fixed Context registry and applies the Chromium Emulation
//! policy to every newly initialized Page or out-of-process Frame session
//! before it is resumed or published.

use std::error::Error;
use std::sync::{Arc, Mutex};

use pwright_backend_cdp_topology::CdpContextOwnership;
use pwright_cdp::{CdpConnection, CdpSession};
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequirements,
    ResourceScope,
};
use pwright_plugin_chromium_session_initialization::{
    ChromiumSessionInitializationFuture, ChromiumSessionInitializationTarget,
    ChromiumSessionInitializer,
};

use crate::HostError;

pub(crate) const MAX_CONTEXTS: usize = 16;
const SCOPE: ResourceScope = ResourceScope::extension(186, 0);

struct State {
    default_disabled: bool,
    contexts: Vec<ObjectId>,
    closed: bool,
}

pub(crate) const fn requirements() -> ResourceRequirements {
    let storage_bytes = (std::mem::size_of::<Installation>()
        + std::mem::size_of::<State>()
        + MAX_CONTEXTS * std::mem::size_of::<ObjectId>()) as u64;
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
    _storage: ResourceLease,
}

impl Installation {
    pub(crate) fn new_with_default(
        resources: &ProductionResourceLedger,
        default_disabled: bool,
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
        Ok(Arc::new(Self {
            state: Mutex::new(State {
                default_disabled,
                contexts,
                closed: false,
            }),
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
                "JavaScript-disabled policy is already installed for this Context".into(),
            ));
        }
        if state.contexts.len() + usize::from(state.default_disabled) == MAX_CONTEXTS {
            return Err(HostError::ResourceExhausted(format!(
                "JavaScript-disabled policy exceeds {MAX_CONTEXTS} live Contexts"
            )));
        }
        state.contexts.push(context);
        Ok(())
    }

    pub(crate) fn remove(&self, context: ObjectId) -> Result<(), HostError> {
        self.state
            .lock()
            .map_err(HostError::backend)?
            .contexts
            .retain(|candidate| *candidate != context);
        Ok(())
    }

    fn disabled(
        &self,
        context: ObjectId,
        ownership: CdpContextOwnership,
    ) -> Result<bool, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        Ok(match ownership {
            CdpContextOwnership::Default => state.default_disabled,
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
    }
}

impl Drop for Installation {
    fn drop(&mut self) {
        self.close();
    }
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
            if !self
                .installation
                .disabled(target.context, target.ownership)?
            {
                return Ok(());
            }
            CdpSession::new(
                Arc::clone(&self.connection),
                target.session_id.to_owned(),
                target.target_id.to_owned(),
            )
            .send(
                "Emulation.setScriptExecutionDisabled",
                serde_json::json!({ "value": true }),
            )
            .await?;
            Ok::<(), Box<dyn Error + Send + Sync>>(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_core::ResourceLimits;

    fn resources() -> ProductionResourceLedger {
        let required = requirements();
        let mut limits = ResourceLimits::deny_all(required.scopes, required.leases);
        for class in ResourceClass::ALL {
            let limit = required.class(class);
            limits = limits.with_class(class, limit.global, limit.per_scope);
        }
        ProductionResourceLedger::new(limits, required)
            .expect("valid JavaScript-disabled policy plan")
    }

    #[test]
    fn policy_is_context_scoped_and_release_is_idempotent() {
        // Arrange
        let installation = Installation::new_with_default(&resources(), false).unwrap();
        let context = ObjectId::new(7);

        // Act
        installation.install(context).unwrap();
        let installed = installation
            .disabled(context, CdpContextOwnership::Owned)
            .unwrap();
        installation.remove(context).unwrap();
        installation.remove(context).unwrap();

        // Assert
        assert!(installed);
        assert!(
            !installation
                .disabled(context, CdpContextOwnership::Owned)
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
        let default_disabled = installation
            .disabled(context, CdpContextOwnership::Default)
            .unwrap();
        let owned_disabled = installation
            .disabled(context, CdpContextOwnership::Owned)
            .unwrap();
        let external_disabled = installation
            .disabled(context, CdpContextOwnership::External)
            .unwrap();

        // Assert
        assert!(default_disabled);
        assert!(!owned_disabled);
        assert!(!external_disabled);
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
}
