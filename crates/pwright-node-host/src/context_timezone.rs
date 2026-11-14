//! Optional Context-scoped Chromium timezone policy.
//!
//! Rust retains one bounded timezone identifier per selected Context and
//! applies it through Chromium's session-initialization fence before a Page or
//! out-of-process Frame is published. Chromium remains the authority for IANA
//! timezone validity; the language bindings only forward bounded strings.

use std::error::Error;
use std::sync::{Arc, Mutex};

use pwright_backend_cdp_topology::CdpContextOwnership;
use pwright_cdp::{CdpConnection, CdpSession, connection::CdpError};
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequest,
    ResourceRequirements, ResourceScope,
};
use pwright_plugin_chromium_session_initialization::{
    ChromiumSessionInitializationFuture, ChromiumSessionInitializationTarget,
    ChromiumSessionInitializer,
};

use crate::HostError;

pub(crate) const MAX_CONTEXTS: usize = 16;
pub(crate) const MAX_TIMEZONE_BYTES: usize = 256;
const MAX_RETAINED_BYTES: u64 = (MAX_CONTEXTS * MAX_TIMEZONE_BYTES) as u64;
const SCOPE: ResourceScope = ResourceScope::extension(192, 0);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextTimezone(Box<str>);

impl ContextTimezone {
    pub fn new(value: String) -> Result<Self, HostError> {
        if value.is_empty() {
            return Err(HostError::InvalidArgument(
                "BrowserContext timezone_id must not be empty".into(),
            ));
        }
        if value.len() > MAX_TIMEZONE_BYTES {
            return Err(HostError::ResourceExhausted(format!(
                "Context timezone identifier exceeds {MAX_TIMEZONE_BYTES} UTF-8 bytes"
            )));
        }
        if value.chars().any(char::is_control) {
            return Err(HostError::InvalidArgument(
                "BrowserContext timezone_id must not contain control characters".into(),
            ));
        }
        Ok(Self(value.into_boxed_str()))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

pub(crate) struct StoredTimezone {
    timezone: ContextTimezone,
    _lease: ResourceLease,
}

impl StoredTimezone {
    fn as_str(&self) -> &str {
        self.timezone.as_str()
    }
}

struct ContextEntry {
    context: ObjectId,
    timezone: Arc<StoredTimezone>,
}

struct State {
    default_timezone: Option<Arc<StoredTimezone>>,
    entries: Vec<ContextEntry>,
    closed: bool,
}

pub(crate) const fn requirements() -> ResourceRequirements {
    let storage_bytes = (std::mem::size_of::<Installation>()
        + std::mem::size_of::<State>()
        + MAX_CONTEXTS * std::mem::size_of::<ContextEntry>()) as u64;
    ResourceRequirements::none()
        .with_class(ResourceClass::EventItem, MAX_CONTEXTS as u64, 1)
        .with_class(
            ResourceClass::EventBytes,
            MAX_RETAINED_BYTES,
            MAX_TIMEZONE_BYTES as u64,
        )
        .with_class(
            ResourceClass::BackendStateBytes,
            storage_bytes,
            storage_bytes,
        )
        .with_metadata(MAX_CONTEXTS + 1, MAX_CONTEXTS * 3 + 1)
}

pub(crate) struct Installation {
    resources: pwright_core::ResourceLedger,
    state: Mutex<State>,
    _storage: ResourceLease,
}

impl Installation {
    pub(crate) fn new_with_default(
        resources: &ProductionResourceLedger,
        default_timezone: Option<ContextTimezone>,
    ) -> Result<Arc<Self>, HostError> {
        let plan = requirements();
        let ledger = resources.claim(plan).map_err(HostError::backend)?;
        let storage = ledger
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
        let default_timezone = default_timezone
            .map(|timezone| retained_timezone(&ledger, SCOPE, timezone))
            .transpose()?;
        Ok(Arc::new(Self {
            resources: ledger,
            state: Mutex::new(State {
                default_timezone,
                entries,
                closed: false,
            }),
            _storage: storage,
        }))
    }

    pub(crate) fn install(
        &self,
        context: ObjectId,
        timezone: ContextTimezone,
    ) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        if state.entries.iter().any(|entry| entry.context == context) {
            return Err(HostError::InvalidArgument(
                "timezone policy is already installed for this Context".into(),
            ));
        }
        let retained = state
            .entries
            .len()
            .saturating_add(usize::from(state.default_timezone.is_some()));
        if retained >= MAX_CONTEXTS {
            return Err(HostError::ResourceExhausted(format!(
                "timezone policy exceeds {MAX_CONTEXTS} live Contexts"
            )));
        }
        let timezone =
            retained_timezone(&self.resources, ResourceScope::Object(context), timezone)?;
        state.entries.push(ContextEntry { context, timezone });
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

    fn timezone(
        &self,
        context: ObjectId,
        ownership: CdpContextOwnership,
    ) -> Result<Option<Arc<StoredTimezone>>, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        Ok(match ownership {
            CdpContextOwnership::Default => state.default_timezone.as_ref().map(Arc::clone),
            CdpContextOwnership::Owned => state
                .entries
                .iter()
                .find(|entry| entry.context == context)
                .map(|entry| Arc::clone(&entry.timezone)),
            CdpContextOwnership::External => None,
        })
    }

    fn close(&self) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.closed = true;
        state.default_timezone = None;
        state.entries.clear();
    }
}

impl Drop for Installation {
    fn drop(&mut self) {
        self.close();
    }
}

fn retained_timezone(
    resources: &pwright_core::ResourceLedger,
    scope: ResourceScope,
    timezone: ContextTimezone,
) -> Result<Arc<StoredTimezone>, HostError> {
    let bytes = u64::try_from(timezone.as_str().len())
        .map_err(|_| HostError::ResourceExhausted("timezone size overflow".into()))?;
    let lease = resources
        .reserve_many(
            scope,
            &[
                ResourceRequest::new(ResourceClass::EventItem, 1),
                ResourceRequest::new(ResourceClass::EventBytes, bytes),
            ],
        )
        .map_err(HostError::backend)?;
    Ok(Arc::new(StoredTimezone {
        timezone,
        _lease: lease,
    }))
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

    async fn apply(
        &self,
        session_id: &str,
        target_id: &str,
        context: ObjectId,
        ownership: CdpContextOwnership,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let Some(timezone) = self.installation.timezone(context, ownership)? else {
            return Ok(());
        };
        let session = CdpSession::new(
            Arc::clone(&self.connection),
            session_id.to_owned(),
            target_id.to_owned(),
        );
        match session
            .send(
                "Emulation.setTimezoneOverride",
                timezone_override_params(timezone.as_str()),
            )
            .await
        {
            Ok(_) => Ok(()),
            Err(error) if shared_renderer_timezone_is_already_applied(&error) => Ok(()),
            Err(error) if invalid_timezone(&error) => Err(HostError::InvalidArgument(format!(
                "Invalid timezone ID: {}",
                timezone.as_str()
            ))
            .into()),
            Err(error) => Err(error.into()),
        }
    }
}

fn shared_renderer_timezone_is_already_applied(error: &CdpError) -> bool {
    matches!(
        error,
        CdpError::Protocol { message, .. }
            if message.contains("Timezone override is already in effect")
    )
}

fn invalid_timezone(error: &CdpError) -> bool {
    matches!(
        error,
        CdpError::Protocol { message, .. } if message.contains("Invalid timezone")
    )
}

fn timezone_override_params(timezone_id: &str) -> serde_json::Value {
    serde_json::json!({ "timezoneId": timezone_id })
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
            self.apply(
                target.session_id,
                target.target_id,
                target.context,
                target.ownership,
            )
            .await?;
            Ok(())
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
            let bound = required.class(class);
            limits = limits.with_class(class, bound.global, bound.per_scope);
        }
        ProductionResourceLedger::new(limits, required).expect("valid timezone resource plan")
    }

    #[test]
    fn timezone_is_context_scoped_and_release_is_idempotent() {
        // Arrange
        let installation = Installation::new_with_default(&resources(), None).unwrap();
        let context = ObjectId::new(7);
        let timezone = ContextTimezone::new("America/Jamaica".to_owned()).unwrap();

        // Act
        installation.install(context, timezone).unwrap();
        let observed = installation
            .timezone(context, CdpContextOwnership::Owned)
            .unwrap()
            .unwrap();
        installation.remove(context).unwrap();
        installation.remove(context).unwrap();

        // Assert
        assert_eq!(observed.as_str(), "America/Jamaica");
        assert!(
            installation
                .timezone(context, CdpContextOwnership::Owned)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn default_timezone_is_selected_only_for_the_default_context() {
        // Arrange
        let timezone = ContextTimezone::new("America/Jamaica".to_owned()).unwrap();
        let installation = Installation::new_with_default(&resources(), Some(timezone)).unwrap();
        let context = ObjectId::new(7);

        // Act
        let default = installation
            .timezone(context, CdpContextOwnership::Default)
            .unwrap();
        let owned = installation
            .timezone(context, CdpContextOwnership::Owned)
            .unwrap();
        let external = installation
            .timezone(context, CdpContextOwnership::External)
            .unwrap();

        // Assert
        assert_eq!(default.unwrap().as_str(), "America/Jamaica");
        assert!(owned.is_none());
        assert!(external.is_none());
    }

    #[test]
    fn default_timezone_consumes_one_fixed_context_slot() {
        // Arrange
        let timezone = ContextTimezone::new("UTC".to_owned()).unwrap();
        let installation = Installation::new_with_default(&resources(), Some(timezone)).unwrap();
        for value in 1..MAX_CONTEXTS {
            installation
                .install(
                    ObjectId::new(value as u64),
                    ContextTimezone::new("UTC".to_owned()).unwrap(),
                )
                .unwrap();
        }

        // Act
        let error = installation
            .install(
                ObjectId::new(MAX_CONTEXTS as u64),
                ContextTimezone::new("UTC".to_owned()).unwrap(),
            )
            .unwrap_err();

        // Assert
        assert!(matches!(error, HostError::ResourceExhausted(_)));
    }

    #[test]
    fn timezone_identifier_and_protocol_payload_are_bounded_and_exact() {
        // Arrange
        let too_long = "a".repeat(MAX_TIMEZONE_BYTES + 1);

        // Act
        let error = ContextTimezone::new(too_long).unwrap_err();
        let payload = timezone_override_params("America/Jamaica");

        // Assert
        assert!(matches!(error, HostError::ResourceExhausted(_)));
        assert_eq!(
            payload,
            serde_json::json!({ "timezoneId": "America/Jamaica" })
        );
    }
}
