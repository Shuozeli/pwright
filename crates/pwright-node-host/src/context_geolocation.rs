//! Optional Context-scoped Chromium geolocation policy.
//!
//! Rust retains one bounded coordinate triple per selected Context and applies
//! it through Chromium's session-initialization fence before a Page or OOPIF is
//! published. Permission grants are an independent capability.

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
const SCOPE: ResourceScope = ResourceScope::extension(194, 0);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContextGeolocation {
    longitude: f64,
    latitude: f64,
    accuracy: f64,
}

impl ContextGeolocation {
    pub fn new(longitude: f64, latitude: f64, accuracy: f64) -> Result<Self, HostError> {
        if !longitude.is_finite() || !(-180.0..=180.0).contains(&longitude) {
            return Err(HostError::InvalidArgument(
                "geolocation.longitude: precondition -180 <= LONGITUDE <= 180 failed.".into(),
            ));
        }
        if !latitude.is_finite() || !(-90.0..=90.0).contains(&latitude) {
            return Err(HostError::InvalidArgument(
                "geolocation.latitude: precondition -90 <= LATITUDE <= 90 failed.".into(),
            ));
        }
        if !accuracy.is_finite() || accuracy < 0.0 {
            return Err(HostError::InvalidArgument(
                "geolocation.accuracy: precondition 0 <= ACCURACY failed.".into(),
            ));
        }
        Ok(Self {
            longitude,
            latitude,
            accuracy,
        })
    }
}

struct ContextEntry {
    context: ObjectId,
    geolocation: ContextGeolocation,
}

struct State {
    default_geolocation: Option<ContextGeolocation>,
    entries: Vec<ContextEntry>,
    closed: bool,
}

pub(crate) const fn requirements() -> ResourceRequirements {
    let storage_bytes = (std::mem::size_of::<Installation>()
        + std::mem::size_of::<State>()
        + MAX_CONTEXTS * std::mem::size_of::<ContextEntry>()) as u64;
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
        default_geolocation: Option<ContextGeolocation>,
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
        Ok(Arc::new(Self {
            state: Mutex::new(State {
                default_geolocation,
                entries,
                closed: false,
            }),
            _storage: storage,
        }))
    }

    pub(crate) fn install(
        &self,
        context: ObjectId,
        geolocation: ContextGeolocation,
    ) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        if state.entries.iter().any(|entry| entry.context == context) {
            return Err(HostError::InvalidArgument(
                "geolocation policy is already installed for this Context".into(),
            ));
        }
        let retained = state
            .entries
            .len()
            .saturating_add(usize::from(state.default_geolocation.is_some()));
        if retained >= MAX_CONTEXTS {
            return Err(HostError::ResourceExhausted(format!(
                "geolocation policy exceeds {MAX_CONTEXTS} live Contexts"
            )));
        }
        state.entries.push(ContextEntry {
            context,
            geolocation,
        });
        Ok(())
    }

    pub(crate) fn remove(&self, context: ObjectId) -> Result<(), HostError> {
        self.state
            .lock()
            .map_err(HostError::backend)?
            .entries
            .retain(|entry| entry.context != context);
        Ok(())
    }

    fn geolocation(
        &self,
        context: ObjectId,
        ownership: CdpContextOwnership,
    ) -> Result<Option<ContextGeolocation>, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        Ok(match ownership {
            CdpContextOwnership::Default => state.default_geolocation,
            CdpContextOwnership::Owned => state
                .entries
                .iter()
                .find(|entry| entry.context == context)
                .map(|entry| entry.geolocation),
            CdpContextOwnership::External => None,
        })
    }

    fn close(&self) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.closed = true;
        state.default_geolocation = None;
        state.entries.clear();
    }
}

impl Drop for Installation {
    fn drop(&mut self) {
        self.close();
    }
}

fn geolocation_payload(geolocation: ContextGeolocation) -> serde_json::Value {
    serde_json::json!({
        "longitude": geolocation.longitude,
        "latitude": geolocation.latitude,
        "accuracy": geolocation.accuracy
    })
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
            let Some(geolocation) = self
                .installation
                .geolocation(target.context, target.ownership)?
            else {
                return Ok(());
            };
            CdpSession::new(
                Arc::clone(&self.connection),
                target.session_id.to_owned(),
                target.target_id.to_owned(),
            )
            .send(
                "Emulation.setGeolocationOverride",
                geolocation_payload(geolocation),
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
        ProductionResourceLedger::new(limits, required).expect("valid geolocation policy plan")
    }

    fn location() -> ContextGeolocation {
        ContextGeolocation::new(10.0, 20.0, 0.0).unwrap()
    }

    #[test]
    fn coordinates_are_finite_and_bounded() {
        // Arrange
        let cases = [
            (181.0, 0.0, 0.0),
            (0.0, 91.0, 0.0),
            (0.0, 0.0, -1.0),
            (f64::NAN, 0.0, 0.0),
        ];

        // Act
        let outcomes = cases.map(|(longitude, latitude, accuracy)| {
            ContextGeolocation::new(longitude, latitude, accuracy)
        });

        // Assert
        assert!(outcomes.into_iter().all(|outcome| outcome.is_err()));
    }

    #[test]
    fn policy_is_context_scoped_and_release_is_idempotent() {
        // Arrange
        let installation = Installation::new_with_default(&resources(), None).unwrap();
        let context = ObjectId::new(7);

        // Act
        installation.install(context, location()).unwrap();
        let installed = installation
            .geolocation(context, CdpContextOwnership::Owned)
            .unwrap();
        installation.remove(context).unwrap();
        installation.remove(context).unwrap();

        // Assert
        assert_eq!(installed, Some(location()));
        assert_eq!(
            installation
                .geolocation(context, CdpContextOwnership::Owned)
                .unwrap(),
            None
        );
    }

    #[test]
    fn policy_rejects_more_than_the_fixed_context_capacity() {
        // Arrange
        let installation = Installation::new_with_default(&resources(), None).unwrap();
        for value in 1..=MAX_CONTEXTS {
            installation
                .install(ObjectId::new(value as u64), location())
                .unwrap();
        }

        // Act
        let error = installation
            .install(ObjectId::new((MAX_CONTEXTS + 1) as u64), location())
            .unwrap_err();

        // Assert
        assert!(matches!(error, HostError::ResourceExhausted(_)));
    }

    #[test]
    fn default_policy_is_exact_and_consumes_one_slot() {
        // Arrange
        let installation = Installation::new_with_default(&resources(), Some(location())).unwrap();
        for value in 1..MAX_CONTEXTS {
            installation
                .install(ObjectId::new(value as u64), location())
                .unwrap();
        }
        let context = ObjectId::new(99);

        // Act
        let default = installation
            .geolocation(context, CdpContextOwnership::Default)
            .unwrap();
        let external = installation
            .geolocation(context, CdpContextOwnership::External)
            .unwrap();
        let error = installation.install(context, location()).unwrap_err();

        // Assert
        assert_eq!(default, Some(location()));
        assert_eq!(external, None);
        assert!(matches!(error, HostError::ResourceExhausted(_)));
    }

    #[test]
    fn protocol_payload_matches_playwright_chromium() {
        // Arrange
        let expected = serde_json::json!({
            "longitude": 10.0,
            "latitude": 20.0,
            "accuracy": 0.0
        });

        // Act
        let payload = geolocation_payload(location());

        // Assert
        assert_eq!(payload, expected);
    }
}
