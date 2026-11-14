//! Optional geolocation permission for a Chromium BrowserContext.
//!
//! The permission grant is deliberately separate from the geolocation value:
//! callers can install either capability without pulling in the other. Rust
//! bounds selected Context ownership and sends the exact browser-session CDP
//! command before the Context can be returned to a language binding.

use std::sync::{Arc, Mutex};

use pwright_cdp::{CdpConnection, CdpSession};
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequirements,
    ResourceScope,
};

use crate::HostError;

pub(crate) const MAX_CONTEXTS: usize = 16;
const SCOPE: ResourceScope = ResourceScope::extension(195, 0);

struct State {
    default_enabled: bool,
    contexts: Vec<ObjectId>,
    closed: bool,
}

impl State {
    fn admit(&mut self, context: ObjectId) -> Result<(), HostError> {
        if self.closed {
            return Err(HostError::TargetClosed);
        }
        if self.contexts.contains(&context) {
            return Err(HostError::InvalidArgument(
                "geolocation permission is already installed for this Context".into(),
            ));
        }
        if self.contexts.len() + usize::from(self.default_enabled) == MAX_CONTEXTS {
            return Err(HostError::ResourceExhausted(format!(
                "geolocation permission exceeds {MAX_CONTEXTS} live Contexts"
            )));
        }
        self.contexts.push(context);
        Ok(())
    }

    fn remove(&mut self, context: ObjectId) {
        self.contexts.retain(|candidate| *candidate != context);
    }
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
    connection: Arc<CdpConnection>,
    state: Mutex<State>,
    _storage: ResourceLease,
}

impl Installation {
    pub(crate) fn new_with_default(
        resources: &ProductionResourceLedger,
        connection: Arc<CdpConnection>,
        default_enabled: bool,
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
            connection,
            state: Mutex::new(State {
                default_enabled,
                contexts,
                closed: false,
            }),
            _storage: storage,
        }))
    }

    pub(crate) async fn apply_default(&self) -> Result<(), HostError> {
        let enabled = {
            let state = self.state.lock().map_err(HostError::backend)?;
            if state.closed {
                return Err(HostError::TargetClosed);
            }
            state.default_enabled
        };
        if enabled {
            self.grant(None).await?;
        }
        Ok(())
    }

    pub(crate) async fn install(
        &self,
        context: ObjectId,
        browser_context_id: &str,
    ) -> Result<(), HostError> {
        self.state
            .lock()
            .map_err(HostError::backend)?
            .admit(context)?;
        if let Err(error) = self.grant(Some(browser_context_id)).await {
            self.remove(context)?;
            return Err(error);
        }
        Ok(())
    }

    pub(crate) fn remove(&self, context: ObjectId) -> Result<(), HostError> {
        self.state
            .lock()
            .map_err(HostError::backend)?
            .remove(context);
        Ok(())
    }

    async fn grant(&self, browser_context_id: Option<&str>) -> Result<(), HostError> {
        CdpSession::browser(Arc::clone(&self.connection))
            .send(
                "Browser.grantPermissions",
                grant_payload(browser_context_id),
            )
            .await
            .map_err(HostError::backend)?;
        Ok(())
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

fn grant_payload(browser_context_id: Option<&str>) -> serde_json::Value {
    let mut payload = serde_json::json!({ "permissions": ["geolocation"] });
    if let Some(browser_context_id) = browser_context_id {
        payload.as_object_mut().expect("object literal").insert(
            "browserContextId".into(),
            serde_json::Value::String(browser_context_id.to_owned()),
        );
    }
    payload
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(default_enabled: bool) -> State {
        State {
            default_enabled,
            contexts: Vec::with_capacity(MAX_CONTEXTS),
            closed: false,
        }
    }

    #[test]
    fn owned_permission_is_bounded_and_release_is_idempotent() {
        // Arrange
        let mut state = state(false);
        for value in 1..=MAX_CONTEXTS {
            state.admit(ObjectId::new(value as u64)).unwrap();
        }

        // Act
        let error = state
            .admit(ObjectId::new((MAX_CONTEXTS + 1) as u64))
            .unwrap_err();
        state.remove(ObjectId::new(1));
        state.remove(ObjectId::new(1));
        let replacement = state.admit(ObjectId::new((MAX_CONTEXTS + 1) as u64));

        // Assert
        assert!(matches!(error, HostError::ResourceExhausted(_)));
        assert!(replacement.is_ok());
    }

    #[test]
    fn default_permission_consumes_one_fixed_context_slot() {
        // Arrange
        let mut state = state(true);
        for value in 1..MAX_CONTEXTS {
            state.admit(ObjectId::new(value as u64)).unwrap();
        }

        // Act
        let error = state.admit(ObjectId::new(MAX_CONTEXTS as u64)).unwrap_err();

        // Assert
        assert!(matches!(error, HostError::ResourceExhausted(_)));
    }

    #[test]
    fn default_payload_omits_browser_context_id() {
        // Arrange
        let expected = serde_json::json!({ "permissions": ["geolocation"] });

        // Act
        let payload = grant_payload(None);

        // Assert
        assert_eq!(payload, expected);
    }

    #[test]
    fn owned_payload_matches_playwright_chromium() {
        // Arrange
        let expected = serde_json::json!({
            "permissions": ["geolocation"],
            "browserContextId": "context-7"
        });

        // Act
        let payload = grant_payload(Some("context-7"));

        // Assert
        assert_eq!(payload, expected);
    }
}
