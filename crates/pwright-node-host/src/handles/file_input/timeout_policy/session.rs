//! Canonical session-scoped client metadata, never native-use authority.

use super::{TimeoutPolicyBudget, TimeoutSettings};
use crate::{
    HostError, HostSession,
    handles::{HandleSourceIdentity, pending::Owners},
};
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceLedger,
    ResourceLimits, ResourceRequirements, ResourceScope,
};
use std::sync::{Arc, Mutex, Weak};

pub(super) const MAX_SESSIONS: usize = 16;
const MEMBERS: usize = 16;
const INDEX_BYTES: u64 = 16 * 1024;
const SCOPE: ResourceScope = ResourceScope::extension(174, 1);

pub(super) fn index_resources() -> Result<ResourceLedger, HostError> {
    let bytes = MAX_SESSIONS as u64 * INDEX_BYTES;
    let requirements = ResourceRequirements::none()
        .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
        .with_metadata(1, MAX_SESSIONS);
    let limits = ResourceLimits::deny_all(1, MAX_SESSIONS).with_class(
        ResourceClass::BackendStateBytes,
        bytes,
        bytes,
    );
    ProductionResourceLedger::new(limits, requirements)
        .and_then(|ledger| ledger.claim(requirements))
        .map_err(HostError::backend)
}

pub(super) struct SessionSlot {
    origin: Weak<Owners>,
    session: Weak<SessionInner>,
}

struct ContextEntry {
    id: ObjectId,
    settings: TimeoutSettings,
}
struct PageEntry {
    id: ObjectId,
    context: ObjectId,
    settings: TimeoutSettings,
}
struct Index {
    closed: bool,
    contexts: [Option<ContextEntry>; MEMBERS],
    pages: [Option<PageEntry>; MEMBERS],
}
struct SessionInner {
    origin: Weak<Owners>,
    debug: bool,
    index: Mutex<Index>,
    _storage: ResourceLease,
    // Shared administrative admission outlives the final index lease.
    budget: TimeoutPolicyBudget,
}

// Covers the fixed index, Arc header, lease request storage and one synchronous
// membership/pruning operation's detached arrays. No native routes are stored.
const _: () = assert!(
    std::mem::size_of::<SessionInner>()
        + 2 * std::mem::size_of::<usize>()
        + std::mem::size_of::<pwright_core::ResourceRequest>()
        + std::mem::size_of::<Membership>()
        + std::mem::size_of::<Index>()
        < INDEX_BYTES as usize
);

/// One canonical index per exact host origin in this environment budget.
/// Close releases canonical cell ownership. The fixed index admission lasts
/// until the final clone drops; retaining a closed receipt is not a free slot.
#[derive(Clone)]
pub struct TimeoutPolicySession(Arc<SessionInner>);

#[derive(Clone, Copy)]
enum Scope {
    Context,
    Page(ObjectId),
}

/// Client settings with immutable origin and Page provenance, not permission
/// to use a browser object. No strong session/index/native owner is retained.
#[derive(Clone)]
pub struct ScopedTimeoutSettings {
    origin: Weak<Owners>,
    scope: Scope,
    settings: TimeoutSettings,
    _binding: Arc<BindingStorage>,
}

struct BindingStorage {
    _storage: ResourceLease,
    _budget: TimeoutPolicyBudget,
}

const _: () = assert!(
    std::mem::size_of::<BindingStorage>()
        + std::mem::size_of::<ScopedTimeoutSettings>()
        + 2 * std::mem::size_of::<usize>()
        + std::mem::size_of::<pwright_core::ResourceRequest>()
        < super::CELL_BYTES as usize
);

impl ScopedTimeoutSettings {
    pub fn settings(&self) -> &TimeoutSettings {
        &self.settings
    }

    pub fn is_page(&self) -> bool {
        matches!(self.scope, Scope::Page(_))
    }

    /// Pure immutable provenance. Generic, disposed and closed-source Handles
    /// remain valid metadata clients, but missing/wrong Page proof never does.
    pub fn validate_source(&self, source: &HandleSourceIdentity) -> Result<(), HostError> {
        let Scope::Page(page) = self.scope else {
            return Err(invalid("Handle timeout policy must belong to a Page"));
        };
        if !source.belongs_to(&self.origin) {
            return Err(invalid("Handle timeout policy belongs to another session"));
        }
        if source.creation_page_id() != Some(page) {
            return Err(invalid(
                "Handle timeout policy does not match its creation Page",
            ));
        }
        Ok(())
    }
}

fn invalid(message: &str) -> HostError {
    HostError::InvalidArgument(message.into())
}

#[derive(Default)]
struct Membership {
    contexts: [Option<ObjectId>; MEMBERS],
    pages: [Option<(ObjectId, ObjectId)>; MEMBERS],
}
impl Membership {
    fn contains_context(&self, id: ObjectId) -> bool {
        self.contexts.contains(&Some(id))
    }
    fn page_context(&self, id: ObjectId) -> Option<ObjectId> {
        self.pages
            .iter()
            .flatten()
            .find_map(|(page, context)| (*page == id).then_some(*context))
    }
}

impl HostSession {
    fn timeout_policy_origin(&self) -> Result<Weak<Owners>, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        let live = state.as_ref().ok_or(HostError::TargetClosed)?;
        if self.stop.is_cancelled() || self.disconnected.is_cancelled() {
            return Err(HostError::TargetClosed);
        }
        Ok(Arc::downgrade(&live.handles.owners))
    }

    fn timeout_policy_membership(&self) -> Result<Membership, HostError> {
        let view = self
            .frame_publications()
            .published_collections()?
            .ok_or(HostError::TargetClosed)?;
        let mut result = Membership::default();
        for (index, context) in view.contexts().iter().enumerate() {
            *result.contexts.get_mut(index).ok_or_else(|| {
                HostError::ResourceExhausted("timeout Context membership exceeds 16".into())
            })? = Some(context.id);
        }
        for (index, page) in view.pages().iter().enumerate() {
            *result.pages.get_mut(index).ok_or_else(|| {
                HostError::ResourceExhausted("timeout Page membership exceeds 16".into())
            })? = Some((page.id, page.context));
        }
        Ok(result)
    }
}

impl TimeoutPolicyBudget {
    pub fn session(
        &self,
        host: &HostSession,
        debug: bool,
    ) -> Result<TimeoutPolicySession, HostError> {
        self.session_for_origin(host.timeout_policy_origin()?, debug)
    }

    fn session_for_origin(
        &self,
        origin: Weak<Owners>,
        debug: bool,
    ) -> Result<TimeoutPolicySession, HostError> {
        let mut sessions = self.inner.sessions.lock().map_err(HostError::backend)?;
        let existing = sessions.iter().flatten().find_map(|slot| {
            if Weak::ptr_eq(&slot.origin, &origin) {
                slot.session.upgrade()
            } else {
                None
            }
        });
        if let Some(session) = existing {
            // Drop the cache lock before a returned temporary could become
            // the final strong owner and remove its own cache slot.
            drop(sessions);
            if session.debug != debug {
                return Err(invalid("timeout session debug mode is immutable"));
            }
            return Ok(TimeoutPolicySession(session));
        }
        let slot = sessions
            .iter_mut()
            .find(|slot| slot.is_none())
            .ok_or_else(|| {
                HostError::ResourceExhausted("timeout policy session capacity exceeds 16".into())
            })?;
        let storage = self
            .inner
            .indexes
            .reserve(SCOPE, ResourceClass::BackendStateBytes, INDEX_BYTES)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        let session = Arc::new(SessionInner {
            origin: origin.clone(),
            debug,
            index: Mutex::new(Index {
                closed: false,
                contexts: std::array::from_fn(|_| None),
                pages: std::array::from_fn(|_| None),
            }),
            _storage: storage,
            budget: self.clone(),
        });
        *slot = Some(SessionSlot {
            origin,
            session: Arc::downgrade(&session),
        });
        Ok(TimeoutPolicySession(session))
    }
}

impl TimeoutPolicySession {
    pub fn context(
        &self,
        host: &HostSession,
        context_id: &str,
    ) -> Result<ScopedTimeoutSettings, HostError> {
        let id = parse_id(context_id)?;
        self.check_origin(&host.timeout_policy_origin()?)?;
        self.lookup(&host.timeout_policy_membership()?, id, false)
    }

    pub fn page(
        &self,
        host: &HostSession,
        page_id: &str,
    ) -> Result<ScopedTimeoutSettings, HostError> {
        let id = parse_id(page_id)?;
        self.check_origin(&host.timeout_policy_origin()?)?;
        self.lookup(&host.timeout_policy_membership()?, id, true)
    }

    fn check_origin(&self, origin: &Weak<Owners>) -> Result<(), HostError> {
        if !Weak::ptr_eq(&self.0.origin, origin) {
            return Err(invalid("timeout policy index belongs to another session"));
        }
        Ok(())
    }

    fn lookup(
        &self,
        membership: &Membership,
        id: ObjectId,
        page: bool,
    ) -> Result<ScopedTimeoutSettings, HostError> {
        let mut index = self.0.index.lock().map_err(HostError::backend)?;
        if index.closed {
            return Err(HostError::TargetClosed);
        }
        for entry in &mut index.pages {
            if entry
                .as_ref()
                .is_some_and(|entry| membership.page_context(entry.id) != Some(entry.context))
            {
                *entry = None;
            }
        }
        for entry in &mut index.contexts {
            if entry
                .as_ref()
                .is_some_and(|entry| !membership.contains_context(entry.id))
            {
                *entry = None;
            }
        }
        let context = if page {
            membership.page_context(id).ok_or(HostError::TargetClosed)?
        } else {
            id
        };
        if !membership.contains_context(context) {
            return Err(HostError::TargetClosed);
        }
        // Even canonical cells can produce distinct native publication Boxes.
        // Admit the returned binding before committing any new indexed cell.
        let storage = self
            .0
            .budget
            .inner
            .resources
            .reserve(
                super::SCOPE,
                ResourceClass::BackendStateBytes,
                super::CELL_BYTES,
            )
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        let binding = Arc::new(BindingStorage {
            _storage: storage,
            _budget: self.0.budget.clone(),
        });
        if page && let Some(entry) = index.pages.iter().flatten().find(|entry| entry.id == id) {
            return Ok(self.scoped(entry.settings.clone(), Scope::Page(id), binding));
        }
        let existing_context = index
            .contexts
            .iter()
            .flatten()
            .find(|entry| entry.id == context)
            .map(|entry| entry.settings.clone());
        let new_context = existing_context.is_none();
        let settings = match existing_context {
            Some(settings) => settings,
            None => self.0.budget.root(self.0.debug)?,
        };
        // Admit both new cells before committing either index entry. A failed
        // child admission must not retain an otherwise unobserved new root.
        let page_settings = if page { Some(settings.child()?) } else { None };
        if new_context {
            let slot = index
                .contexts
                .iter_mut()
                .find(|entry| entry.is_none())
                .ok_or_else(|| {
                    HostError::ResourceExhausted("timeout Context index exceeds 16".into())
                })?;
            *slot = Some(ContextEntry {
                id: context,
                settings: settings.clone(),
            });
        }
        match page_settings {
            Some(settings) => {
                let slot = index
                    .pages
                    .iter_mut()
                    .find(|entry| entry.is_none())
                    .ok_or_else(|| {
                        HostError::ResourceExhausted("timeout Page index exceeds 16".into())
                    })?;
                *slot = Some(PageEntry {
                    id,
                    context,
                    settings: settings.clone(),
                });
                Ok(self.scoped(settings, Scope::Page(id), binding))
            }
            None => Ok(self.scoped(settings, Scope::Context, binding)),
        }
    }

    fn scoped(
        &self,
        settings: TimeoutSettings,
        scope: Scope,
        binding: Arc<BindingStorage>,
    ) -> ScopedTimeoutSettings {
        ScopedTimeoutSettings {
            origin: self.0.origin.clone(),
            scope,
            settings,
            _binding: binding,
        }
    }

    pub fn close(&self) {
        self.0.close();
    }
}

impl SessionInner {
    fn close(&self) {
        let removed = {
            let mut index = self
                .index
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            index.closed = true;
            (
                std::mem::replace(&mut index.pages, std::array::from_fn(|_| None)),
                std::mem::replace(&mut index.contexts, std::array::from_fn(|_| None)),
            )
        };
        drop(removed);
    }
}

impl Drop for SessionInner {
    fn drop(&mut self) {
        self.close();
        let mut sessions = self
            .budget
            .inner
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for slot in sessions.iter_mut() {
            if slot
                .as_ref()
                .is_some_and(|entry| std::ptr::eq(entry.session.as_ptr(), self))
            {
                *slot = None;
                break;
            }
        }
    }
}

fn parse_id(value: &str) -> Result<ObjectId, HostError> {
    if value.is_empty() || value.len() > 20 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid("invalid timeout policy object ID"));
    }
    value
        .parse::<u64>()
        .map(ObjectId::new)
        .map_err(|_| invalid("invalid timeout policy object ID"))
}

#[cfg(test)]
mod tests;
