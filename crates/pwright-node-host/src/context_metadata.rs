//! Explicit, bounded Context settings. Retained snapshots/bindings contain no
//! topology, registry, HostSession, browser transport or execution runtime.
use std::sync::{Arc, Mutex};

use crate::HostError;
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceLedger,
    ResourceLimits, ResourceRequirements, ResourceScope,
};
use url::Url;

const SCOPE: ResourceScope = ResourceScope::extension(121, 4);
const NAVIGATION_URL_BYTES: usize = 16 * 1024;
const URL_RESOLUTION_BYTES: usize = 32 * 1024;

#[derive(Clone, Copy)]
pub struct ContextMetadataLimits {
    pub active_contexts: usize,
    /// Combined settings objects and retained native bindings.
    pub states: usize,
    pub conversions: usize,
    pub base_url_bytes: usize,
    pub bytes: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ContextMetadataMetrics {
    pub states: u64,
    pub conversions: u64,
    pub bytes: u64,
    pub active_contexts: usize,
    pub pending_creations: usize,
}

struct Storage {
    _count: ResourceLease,
    _bytes: ResourceLease,
}
pub struct ContextConversion(Storage);
impl ContextConversion {
    pub fn finish(self) {
        drop(self.0);
    }
}

fn exhausted(error: impl std::fmt::Display) -> HostError {
    HostError::ResourceExhausted(error.to_string())
}
fn reserve(
    resources: &ResourceLedger,
    class: ResourceClass,
    bytes: usize,
) -> Result<Storage, HostError> {
    let count = resources.reserve(SCOPE, class, 1).map_err(exhausted)?;
    let bytes = resources
        .reserve(
            SCOPE,
            ResourceClass::BackendStateBytes,
            u64::try_from(bytes).map_err(exhausted)?,
        )
        .map_err(exhausted)?;
    Ok(Storage {
        _count: count,
        _bytes: bytes,
    })
}
fn conversion(resources: &ResourceLedger, bytes: usize) -> Result<ContextConversion, HostError> {
    let bytes = bytes
        .checked_add(std::mem::size_of::<ContextConversion>())
        .ok_or_else(|| exhausted("Context conversion storage overflow"))?;
    Ok(ContextConversion(reserve(
        resources,
        ResourceClass::EventItem,
        bytes,
    )?))
}

struct Settings {
    base_url: Box<str>,
    _storage: Storage,
}
#[derive(Clone)]
pub struct ContextSnapshot {
    settings: Option<Arc<Settings>>,
    resources: ResourceLedger,
}
impl std::fmt::Debug for ContextSnapshot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // baseURL may contain credentials; metadata diagnostics do not echo it.
        formatter
            .debug_struct("ContextSnapshot")
            .field("configured", &self.settings.is_some())
            .finish()
    }
}
impl PartialEq for ContextSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.base_url() == other.base_url()
    }
}
impl Eq for ContextSnapshot {}
impl ContextSnapshot {
    pub(crate) fn shares_metadata_with(&self, other: &Self) -> bool {
        self.resources.shares_budget_with(&other.resources)
            && match (&self.settings, &other.settings) {
                (Some(left), Some(right)) => Arc::ptr_eq(left, right),
                (None, None) => true,
                (Some(_), None) | (None, Some(_)) => false,
            }
    }

    pub fn base_url(&self) -> Option<&str> {
        self.settings.as_ref().map(|value| value.base_url.as_ref())
    }
    /// Apply Playwright's URL-construction behavior inside the Rust host.
    /// Invalid inputs remain unchanged so the browser owns the eventual
    /// navigation error, matching `constructURLBasedOnBaseURL` upstream.
    pub fn resolve_url(&self, input: &str) -> Result<String, HostError> {
        if input.len() > NAVIGATION_URL_BYTES {
            return Err(HostError::InvalidArgument(
                "Navigation URL exceeds byte limit".into(),
            ));
        }
        let _conversion = self.reserve_conversion(URL_RESOLUTION_BYTES)?;
        let resolved = self
            .base_url()
            .and_then(|base| Url::parse(base).ok())
            .and_then(|base| base.join(input).ok())
            .map(|url| url.into())
            .unwrap_or_else(|| input.to_owned());
        if resolved.len() > NAVIGATION_URL_BYTES {
            return Err(HostError::InvalidArgument(
                "Resolved navigation URL exceeds byte limit".into(),
            ));
        }
        Ok(resolved)
    }
    pub fn reserve_conversion(&self, bytes: usize) -> Result<ContextConversion, HostError> {
        conversion(&self.resources, bytes)
    }
    pub fn bind(&self) -> Result<ContextBinding, HostError> {
        let storage = reserve(
            &self.resources,
            ResourceClass::LiveObject,
            std::mem::size_of::<ContextBinding>(),
        )?;
        Ok(ContextBinding {
            snapshot: self.clone(),
            _storage: storage,
        })
    }
}
pub struct ContextBinding {
    snapshot: ContextSnapshot,
    _storage: Storage,
}
impl ContextBinding {
    // A binding owns only metadata and its last-reference admission.
    pub fn snapshot(&self) -> &ContextSnapshot {
        &self.snapshot
    }
}
impl std::fmt::Debug for ContextBinding {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ContextBinding")
            .field("snapshot", &self.snapshot)
            .finish()
    }
}

struct Entry {
    id: ObjectId,
    snapshot: ContextSnapshot,
}
struct State {
    entries: Vec<Entry>,
    reserved: usize,
    closed: bool,
    external: ContextSnapshot,
}
struct Inner {
    resources: ResourceLedger,
    limits: ContextMetadataLimits,
    owned_contexts_only: bool,
    state: Mutex<State>,
    _index: ResourceLease,
}
#[derive(Clone)]
pub struct ContextMetadata(Arc<Inner>);

impl ContextMetadata {
    pub fn new(limits: ContextMetadataLimits) -> Result<Self, HostError> {
        if limits.active_contexts == 0
            || limits.states == 0
            || limits.conversions == 0
            || limits.base_url_bytes == 0
        {
            return Err(HostError::InvalidArgument(
                "Context metadata capacities must be positive".into(),
            ));
        }
        let leases = limits
            .states
            .checked_add(limits.conversions)
            .and_then(|n| n.checked_mul(2))
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| exhausted("Context lease capacity overflow"))?;
        let bytes = limits
            .active_contexts
            .checked_mul(std::mem::size_of::<Entry>())
            .and_then(|n| {
                n.checked_add(std::mem::size_of::<Inner>() + 2 * std::mem::size_of::<usize>())
            })
            .ok_or_else(|| exhausted("Context index storage overflow"))?;
        let requirements = ResourceRequirements::none()
            .with_class(
                ResourceClass::LiveObject,
                limits.states as u64,
                limits.states as u64,
            )
            .with_class(
                ResourceClass::EventItem,
                limits.conversions as u64,
                limits.conversions as u64,
            )
            .with_class(ResourceClass::BackendStateBytes, limits.bytes, limits.bytes)
            .with_metadata(1, leases);
        let bounds = ResourceLimits::deny_all(1, leases)
            .with_class(
                ResourceClass::LiveObject,
                limits.states as u64,
                limits.states as u64,
            )
            .with_class(
                ResourceClass::EventItem,
                limits.conversions as u64,
                limits.conversions as u64,
            )
            .with_class(ResourceClass::BackendStateBytes, limits.bytes, limits.bytes);
        let resources = ProductionResourceLedger::new(bounds, requirements)
            .and_then(|ledger| ledger.claim(requirements))
            .map_err(exhausted)?;
        let index = resources
            .reserve(SCOPE, ResourceClass::BackendStateBytes, bytes as u64)
            .map_err(exhausted)?;
        let external = ContextSnapshot {
            settings: None,
            resources: resources.clone(),
        };
        Ok(Self(Arc::new(Inner {
            resources,
            limits,
            owned_contexts_only: false,
            state: Mutex::new(State {
                entries: Vec::with_capacity(limits.active_contexts),
                reserved: 0,
                closed: false,
                external,
            }),
            _index: index,
        })))
    }

    /// Fresh owned Browser launches keep Chromium's native default Context
    /// internal; only Contexts created by this connection are published.
    #[cfg(any(test, feature = "owned-launch"))]
    pub(crate) fn with_owned_contexts_only(mut self) -> Result<Self, HostError> {
        let inner = Arc::get_mut(&mut self.0).ok_or_else(|| {
            HostError::InvalidArgument(
                "Context visibility must be selected before topology publication".into(),
            )
        })?;
        inner.owned_contexts_only = true;
        Ok(self)
    }

    pub(crate) fn owned_contexts_only(&self) -> bool {
        self.0.owned_contexts_only
    }

    /// Selects one stable settings snapshot for default/external Contexts in
    /// an owned persistent-browser session. This must happen before the book
    /// is cloned into topology publication so identity cannot change later.
    pub fn with_external_base_url(self, base_url: &str) -> Result<Self, HostError> {
        if base_url.len() > self.0.limits.base_url_bytes {
            return Err(exhausted("Context baseURL exceeds metadata byte limit"));
        }
        if Arc::strong_count(&self.0) != 1 {
            return Err(HostError::InvalidArgument(
                "external Context settings must be selected before publication".into(),
            ));
        }
        let bytes = base_url
            .len()
            .checked_add(std::mem::size_of::<Settings>() + 2 * std::mem::size_of::<usize>())
            .ok_or_else(|| exhausted("Context settings storage overflow"))?;
        let storage = reserve(&self.0.resources, ResourceClass::LiveObject, bytes)?;
        let external = ContextSnapshot {
            settings: Some(Arc::new(Settings {
                base_url: base_url.into(),
                _storage: storage,
            })),
            resources: self.0.resources.clone(),
        };
        self.0.state.lock().map_err(HostError::backend)?.external = external;
        Ok(self)
    }

    fn empty(&self) -> ContextSnapshot {
        ContextSnapshot {
            settings: None,
            resources: self.0.resources.clone(),
        }
    }

    /// Acquire every registry/settings/binding admission before native creation.
    pub fn reserve_creation(
        &self,
        base_url: Option<&str>,
    ) -> Result<ContextReservation, HostError> {
        if base_url.is_some_and(|value| value.len() > self.0.limits.base_url_bytes) {
            return Err(exhausted("Context baseURL exceeds metadata byte limit"));
        }
        let storage = conversion(
            &self.0.resources,
            std::mem::size_of::<ContextReservation>() - std::mem::size_of::<ContextConversion>(),
        )?;
        let mut reservation = ContextReservation {
            owner: self.clone(),
            snapshot: self.empty(),
            binding: None,
            registered: false,
            _storage: storage,
        };
        {
            let mut state = self.0.state.lock().map_err(HostError::backend)?;
            if state.closed {
                return Err(HostError::TargetClosed);
            }
            if state.entries.len() + state.reserved >= self.0.limits.active_contexts {
                return Err(exhausted("Context metadata registry is full"));
            }
            state.reserved += 1;
            reservation.registered = true;
        }
        if let Some(value) = base_url {
            let bytes = value
                .len()
                .checked_add(std::mem::size_of::<Settings>() + 2 * std::mem::size_of::<usize>())
                .ok_or_else(|| exhausted("Context settings storage overflow"))?;
            let storage = reserve(&self.0.resources, ResourceClass::LiveObject, bytes)?;
            reservation.snapshot.settings = Some(Arc::new(Settings {
                base_url: value.into(),
                _storage: storage,
            }));
        }
        reservation.binding = Some(reservation.snapshot.bind()?);
        Ok(reservation)
    }

    /// Owned Contexts are hidden until commit. External/default Contexts have
    /// no locally configured settings and need no registry entry.
    pub fn visible(&self, id: ObjectId, owned: bool) -> Result<Option<ContextSnapshot>, HostError> {
        if !owned && self.0.owned_contexts_only {
            return Ok(None);
        }
        let state = self.0.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Ok(None);
        }
        Ok(state
            .entries
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.snapshot.clone())
            .or_else(|| (!owned).then(|| state.external.clone())))
    }

    pub fn retain_live(&self, mut present: impl FnMut(ObjectId) -> bool) -> Result<(), HostError> {
        self.0
            .state
            .lock()
            .map_err(HostError::backend)?
            .entries
            .retain(|entry| present(entry.id));
        Ok(())
    }
    pub fn remove(&self, id: ObjectId) -> Result<(), HostError> {
        self.retain_live(|entry| entry != id)
    }
    pub fn clear(&self) {
        let external = self.empty();
        let mut state = self
            .0
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.closed = true;
        state.entries.clear();
        state.external = external;
        // Outstanding reservations still decrement their own counts on Drop.
    }
    pub fn reserve_conversion(&self, bytes: usize) -> Result<ContextConversion, HostError> {
        conversion(&self.0.resources, bytes)
    }
    pub fn metrics(&self) -> Result<ContextMetadataMetrics, HostError> {
        let state = self.0.state.lock().map_err(HostError::backend)?;
        let metrics = self.0.resources.metrics().map_err(exhausted)?;
        Ok(ContextMetadataMetrics {
            states: metrics.class(ResourceClass::LiveObject).current,
            conversions: metrics.class(ResourceClass::EventItem).current,
            bytes: metrics.class(ResourceClass::BackendStateBytes).current,
            active_contexts: state.entries.len(),
            pending_creations: state.reserved,
        })
    }
}

pub struct ContextReservation {
    owner: ContextMetadata,
    snapshot: ContextSnapshot,
    binding: Option<ContextBinding>,
    registered: bool,
    _storage: ContextConversion,
}
pub struct CreatedContextMetadata {
    pub snapshot: ContextSnapshot,
    pub binding: ContextBinding,
}
impl ContextReservation {
    pub fn commit(mut self, id: ObjectId) -> Result<CreatedContextMetadata, HostError> {
        let mut state = self.owner.0.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        if id.get() == 0 || state.entries.iter().any(|entry| entry.id == id) {
            return Err(HostError::InvalidArgument(
                "duplicate or invalid Context metadata identity".into(),
            ));
        }
        state.entries.push(Entry {
            id,
            snapshot: self.snapshot.clone(),
        });
        state.reserved -= 1;
        self.registered = false;
        Ok(CreatedContextMetadata {
            snapshot: self.snapshot.clone(),
            binding: self
                .binding
                .take()
                .expect("creation pre-admitted its binding"),
        })
    }
}
impl Drop for ContextReservation {
    fn drop(&mut self) {
        if self.registered {
            let mut state = self
                .owner
                .0
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.reserved -= 1;
        }
    }
}

#[cfg(test)]
mod tests;
