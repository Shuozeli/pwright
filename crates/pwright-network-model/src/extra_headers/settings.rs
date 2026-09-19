//! Bounded metadata transactions for an embedding host's native writers.
//! No transport or executor is installed here. A rollback acknowledgement must
//! follow native compensation; dropping an unresolved update restores desired
//! metadata but records native uncertainty separately, without blocking reads.
use super::{ContextHeaders, EffectiveHeaders, HeaderError, HeaderPool, HeaderRef, PageHeaders};
use pwright_core::{
    ObjectId, ProductionResourceError, ProductionResourceLedger, ResourceClass, ResourceError,
    ResourceLease, ResourceRequirements, ResourceScope,
};
use std::{
    fmt,
    sync::{Arc, Mutex, MutexGuard},
};

#[derive(Debug)]
pub enum SettingsError {
    InvalidLimits,
    InvalidIdentity,
    Capacity,
    Busy,
    Closed,
    Superseded,
    InvalidResolution,
    ParentMismatch,
    StateUnavailable,
    Headers(HeaderError),
    Budget(ResourceError),
    Plan(ProductionResourceError),
}
impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimits => f.write_str("invalid header settings profile"),
            Self::InvalidIdentity => f.write_str("invalid header settings identity"),
            Self::Capacity => f.write_str("header settings capacity exhausted"),
            Self::Busy => f.write_str("header settings writer is already active"),
            Self::Closed => f.write_str("header settings registry is closed"),
            Self::Superseded => f.write_str("header settings update was retired"),
            Self::InvalidResolution => f.write_str("cannot commit a restoring header update"),
            Self::ParentMismatch => f.write_str("header Page has a different owning Context"),
            Self::StateUnavailable => f.write_str("header settings state unavailable"),
            Self::Headers(error) => error.fmt(f),
            Self::Budget(error) => error.fmt(f),
            Self::Plan(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for SettingsError {}

/// Status of the registry's recorded writes, not a proof that a newly attached
/// native session has been initialized. Desired metadata remains readable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeaderUpdateStatus {
    Stable,
    Applying,
    Restoring,
    Indeterminate,
}

#[derive(Clone, Copy, Debug)]
pub struct HeaderSettingsProfile {
    scope: ResourceScope,
    contexts: usize,
    pages: usize,
    writers: usize,
    bytes: u64,
}
impl HeaderSettingsProfile {
    pub fn new(scope: ResourceScope, contexts: usize, pages: usize) -> Result<Self, SettingsError> {
        let slots = contexts
            .checked_add(pages)
            .ok_or(SettingsError::InvalidLimits)?;
        if slots == 0 {
            return Err(SettingsError::InvalidLimits);
        }
        let bytes = profile_bytes(slots, 1)?;
        Ok(Self {
            scope,
            contexts,
            pages,
            writers: 1,
            bytes,
        })
    }
    /// Opt into a bounded number of overlapping native writes. Existing users
    /// retain the one-writer contract unless explicitly selecting this profile.
    pub fn with_concurrent_writers(mut self, writers: usize) -> Result<Self, SettingsError> {
        if writers == 0 {
            return Err(SettingsError::InvalidLimits);
        }
        self.bytes = profile_bytes(self.contexts + self.pages, writers)?;
        self.writers = writers;
        Ok(self)
    }
    pub fn requirements(self) -> ResourceRequirements {
        ResourceRequirements::none()
            .with_class(ResourceClass::BackendStateBytes, self.bytes, self.bytes)
            .with_metadata(1, 1)
    }
}

fn profile_bytes(slots: usize, writers: usize) -> Result<u64, SettingsError> {
    slots
        .checked_mul(std::mem::size_of::<Entry>())
        .and_then(|bytes| {
            writers
                .checked_mul(std::mem::size_of::<Pending>())
                .and_then(|pending| bytes.checked_add(pending))
        })
        .and_then(|bytes| {
            bytes
                .checked_add(std::mem::size_of::<Shared>() + 2 * std::mem::size_of::<usize>() + 512)
        })
        .and_then(|bytes| u64::try_from(bytes).ok())
        .ok_or(SettingsError::InvalidLimits)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    Context(ObjectId),
    Page { context: ObjectId, page: ObjectId },
}
impl Target {
    fn context(self) -> ObjectId {
        match self {
            Self::Context(context) | Self::Page { context, .. } => context,
        }
    }
    fn validate(self) -> Result<(), SettingsError> {
        if self.context().get() == 0 || matches!(self, Self::Page { page, .. } if page.get() == 0) {
            return Err(SettingsError::InvalidIdentity);
        }
        Ok(())
    }
}

#[derive(Clone)]
enum Headers {
    Context(ContextHeaders),
    Page(PageHeaders),
    Absent,
}
impl Headers {
    fn is_empty(&self) -> bool {
        match self {
            Self::Context(value) => value.iter().len() == 0,
            Self::Page(value) => value.iter().len() == 0,
            Self::Absent => true,
        }
    }
}
struct Entry {
    target: Target,
    headers: Headers,
    indeterminate: bool,
    generation: u64,
}
struct Pending {
    generation: u64,
    target: Target,
    prior: Option<Entry>,
    restoring: bool,
}
struct State {
    entries: Vec<Entry>,
    pending: Vec<Pending>,
    generation: u64,
    closed: bool,
}
struct Shared {
    profile: HeaderSettingsProfile,
    state: Mutex<State>,
    pool: HeaderPool,
    _lease: ResourceLease,
}
impl Shared {
    fn state(&self) -> Result<MutexGuard<'_, State>, SettingsError> {
        let state = self
            .state
            .lock()
            .map_err(|_| SettingsError::StateUnavailable)?;
        if state.closed {
            return Err(SettingsError::Closed);
        }
        Ok(state)
    }
}

/// Sparse settings owner. Portable IDs are supplied by the host, not validated
/// against native topology here. The host owns native dispatch/attachment order.
#[derive(Clone)]
pub struct HeaderSettings(Arc<Shared>);
impl HeaderSettings {
    pub fn new(
        profile: HeaderSettingsProfile,
        pool: HeaderPool,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, SettingsError> {
        let ledger = resources
            .claim(profile.requirements())
            .map_err(SettingsError::Plan)?;
        let lease = ledger
            .reserve(
                profile.scope,
                ResourceClass::BackendStateBytes,
                profile.bytes,
            )
            .map_err(SettingsError::Budget)?;
        Ok(Self(Arc::new(Shared {
            profile,
            state: Mutex::new(State {
                entries: Vec::with_capacity(profile.contexts + profile.pages),
                pending: Vec::with_capacity(profile.writers),
                generation: 0,
                closed: false,
            }),
            pool,
            _lease: lease,
        })))
    }

    pub fn begin_context(
        &self,
        context: ObjectId,
        input: &[HeaderRef<'_>],
    ) -> Result<HeaderUpdate, SettingsError> {
        self.begin(Target::Context(context), input)
    }
    pub fn begin_page(
        &self,
        context: ObjectId,
        page: ObjectId,
        input: &[HeaderRef<'_>],
    ) -> Result<HeaderUpdate, SettingsError> {
        self.begin(Target::Page { context, page }, input)
    }
    fn begin(
        &self,
        target: Target,
        input: &[HeaderRef<'_>],
    ) -> Result<HeaderUpdate, SettingsError> {
        target.validate()?;
        let mut state = self.0.state()?;
        if state.pending.len() == self.0.profile.writers {
            return Err(SettingsError::Busy);
        }
        check_parent(&state, target)?;
        let index = state
            .entries
            .iter()
            .position(|entry| entry.target == target);
        if index.is_none() {
            let (count, limit) = match target {
                Target::Context(_) => (
                    state
                        .entries
                        .iter()
                        .filter(|entry| matches!(entry.target, Target::Context(_)))
                        .count(),
                    self.0.profile.contexts,
                ),
                Target::Page { .. } => (
                    state
                        .entries
                        .iter()
                        .filter(|entry| matches!(entry.target, Target::Page { .. }))
                        .count(),
                    self.0.profile.pages,
                ),
            };
            if count >= limit {
                return Err(SettingsError::Capacity);
            }
        }
        let generation = state
            .generation
            .checked_add(1)
            .ok_or(SettingsError::Capacity)?;
        // Admission failure leaves metadata and generation unchanged.
        let headers = match target {
            Target::Context(_) => self.0.pool.context(input).map(Headers::Context),
            Target::Page { .. } => self.0.pool.page(input).map(Headers::Page),
        }
        .map_err(SettingsError::Headers)?;
        let next = Entry {
            target,
            headers,
            indeterminate: false,
            generation,
        };
        let prior = match index {
            Some(index) => Some(std::mem::replace(&mut state.entries[index], next)),
            None => {
                state.entries.push(next);
                None
            }
        };
        state.generation = generation;
        state.pending.push(Pending {
            generation,
            target,
            prior,
            restoring: false,
        });
        Ok(HeaderUpdate {
            shared: self.0.clone(),
            generation,
        })
    }

    /// Proposed settings are visible while the native writer applies
    /// them, matching Playwright's pre-await assignment. No native call occurs.
    pub fn effective(
        &self,
        context: ObjectId,
        page: ObjectId,
    ) -> Result<EffectiveHeaders, SettingsError> {
        let target = Target::Page { context, page };
        target.validate()?;
        let state = self.0.state()?;
        check_parent(&state, target)?;
        let context =
            state
                .entries
                .iter()
                .find_map(|entry| match (&entry.target, &entry.headers) {
                    (Target::Context(id), Headers::Context(headers)) if *id == context => {
                        Some(headers)
                    }
                    _ => None,
                });
        let page = state
            .entries
            .iter()
            .find_map(|entry| match (&entry.target, &entry.headers) {
                (Target::Page { page: id, .. }, Headers::Page(headers)) if *id == page => {
                    Some(headers)
                }
                _ => None,
            });
        self.0
            .pool
            .merge(context, page)
            .map_err(SettingsError::Headers)
    }

    /// Original Page settings, never merged Context values. The returned clone
    /// shares the original admitted owner and can outlive this registry.
    pub fn page_headers(
        &self,
        context: ObjectId,
        page: ObjectId,
    ) -> Result<Option<PageHeaders>, SettingsError> {
        let target = Target::Page { context, page };
        target.validate()?;
        let state = self.0.state()?;
        check_parent(&state, target)?;
        Ok(state
            .entries
            .iter()
            .find_map(|entry| match (&entry.target, &entry.headers) {
                (Target::Page { page: id, .. }, Headers::Page(headers)) if *id == page => {
                    Some(headers.clone())
                }
                _ => None,
            }))
    }

    /// Call only after the embedding host confirms native retirement. A pending
    /// update for this identity is superseded, never restored by its later drop.
    pub fn retire_page(&self, context: ObjectId, page: ObjectId) -> Result<(), SettingsError> {
        let target = Target::Page { context, page };
        target.validate()?;
        let mut state = self.0.state()?;
        check_parent(&state, target)?;
        retire(&mut state, |candidate| candidate == target);
        Ok(())
    }

    pub fn update_status(
        &self,
        context: ObjectId,
        page: ObjectId,
    ) -> Result<HeaderUpdateStatus, SettingsError> {
        let target = Target::Page { context, page };
        target.validate()?;
        let state = self.0.state()?;
        check_parent(&state, target)?;
        if state
            .pending
            .iter()
            .any(|pending| affects(pending.target, target))
        {
            return Ok(
                if state
                    .pending
                    .iter()
                    .any(|pending| pending.restoring && affects(pending.target, target))
                {
                    HeaderUpdateStatus::Restoring
                } else {
                    HeaderUpdateStatus::Applying
                },
            );
        }
        if state
            .entries
            .iter()
            .any(|entry| entry.indeterminate && affects(entry.target, target))
        {
            return Ok(HeaderUpdateStatus::Indeterminate);
        }
        Ok(HeaderUpdateStatus::Stable)
    }
    pub fn retire_context(&self, context: ObjectId) -> Result<(), SettingsError> {
        Target::Context(context).validate()?;
        let mut state = self.0.state()?;
        retire(&mut state, |target| target.context() == context);
        Ok(())
    }

    /// Remove metadata only after the embedding host has reconciled native
    /// topology. Predicates must be read-only and must not reenter this registry.
    /// No ID collection or additional storage is allocated for reconciliation.
    pub fn retain_live(
        &self,
        mut context_live: impl FnMut(ObjectId) -> bool,
        mut page_live: impl FnMut(ObjectId, ObjectId) -> bool,
    ) -> Result<(), SettingsError> {
        let mut state = self.0.state()?;
        retire(&mut state, |target| match target {
            Target::Context(context) => !context_live(context),
            Target::Page { context, page } => !page_live(context, page),
        });
        Ok(())
    }

    /// Metadata shutdown, not permission to disconnect a shared browser.
    pub fn close(&self) -> Result<(), SettingsError> {
        let mut state = self
            .0
            .state
            .lock()
            .map_err(|_| SettingsError::StateUnavailable)?;
        state.closed = true;
        state.pending.clear();
        state.entries.clear();
        Ok(())
    }
}

fn check_parent(state: &State, target: Target) -> Result<(), SettingsError> {
    if let Target::Page { context, page } = target
        && state.entries.iter().any(|entry| matches!(entry.target, Target::Page { context: prior, page: id } if id == page && prior != context)) {
        return Err(SettingsError::ParentMismatch);
    }
    Ok(())
}
fn affects(scope: Target, target: Target) -> bool {
    scope == target || scope == Target::Context(target.context())
}
fn retire(state: &mut State, mut predicate: impl FnMut(Target) -> bool) {
    state.pending.retain(|pending| !predicate(pending.target));
    state.entries.retain(|entry| !predicate(entry.target));
}

/// One admitted in-flight writer. Caller owns native fanout/compensation
/// and the executor/foreign memory. This handle owns only metadata resolution.
pub struct HeaderUpdate {
    shared: Arc<Shared>,
    generation: u64,
}
impl fmt::Debug for HeaderUpdate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HeaderUpdate")
            .field("generation", &self.generation)
            .finish()
    }
}
impl HeaderUpdate {
    /// Read the rollback plan through a separately admitted effective owner.
    /// `restore_prior` makes that plan visible before native compensation starts.
    pub fn prior_effective(
        &self,
        context: ObjectId,
        page: ObjectId,
    ) -> Result<EffectiveHeaders, SettingsError> {
        let target = Target::Page { context, page };
        target.validate()?;
        let state = self.shared.state()?;
        check_parent(&state, target)?;
        let pending = state
            .pending
            .iter()
            .find(|pending| pending.generation == self.generation)
            .ok_or(SettingsError::Superseded)?;
        let context_entry = if pending.target == Target::Context(context) {
            pending.prior.as_ref()
        } else {
            state
                .entries
                .iter()
                .find(|entry| entry.target == Target::Context(context))
        };
        let page_entry = if pending.target == target {
            pending.prior.as_ref()
        } else {
            state.entries.iter().find(|entry| entry.target == target)
        };
        let context = context_entry.and_then(|entry| match &entry.headers {
            Headers::Context(value) => Some(value),
            Headers::Page(_) => None,
            Headers::Absent => None,
        });
        let page = page_entry.and_then(|entry| match &entry.headers {
            Headers::Page(value) => Some(value),
            Headers::Context(_) => None,
            Headers::Absent => None,
        });
        self.shared
            .pool
            .merge(context, page)
            .map_err(SettingsError::Headers)
    }

    /// Restore the previous local metadata before dispatching compensation,
    /// preserving Playwright's visibility order. This does not acknowledge that
    /// native restoration succeeded; the writer remains held until resolution.
    pub fn restore_prior(&self) -> Result<(), SettingsError> {
        let mut state = self.shared.state()?;
        restore(&mut state, self.generation)
    }

    /// Acknowledge that every eligible native session accepted the proposal.
    pub fn commit(self) -> Result<(), SettingsError> {
        self.resolve(false)
    }
    /// Acknowledge that native compensation restored the prior settings (or no
    /// native side effect occurred). This method does not perform compensation.
    pub fn rollback(self) -> Result<(), SettingsError> {
        self.resolve(true)
    }
    fn resolve(&self, rollback: bool) -> Result<(), SettingsError> {
        let mut state = self.shared.state()?;
        let pending_index = state
            .pending
            .iter()
            .position(|pending| pending.generation == self.generation)
            .ok_or(SettingsError::Superseded)?;
        if !rollback && state.pending[pending_index].restoring {
            return Err(SettingsError::InvalidResolution);
        }
        if rollback {
            restore(&mut state, self.generation)?;
        }
        let pending = state.pending.remove(pending_index);
        let index = state
            .entries
            .iter()
            .position(|entry| entry.target == pending.target)
            .ok_or(SettingsError::Superseded)?;
        // A native ACK never publishes its old proposal again, nor repeats a
        // rollback assignment that already happened before compensation. A later
        // setter (or another failure) may have changed desired state meanwhile.
        if state.entries[index].generation == self.generation {
            state.entries[index].indeterminate = false;
        }
        // A confirmed Context-wide application or compensation includes the
        // complete effective Page headers only for the ordered writer profile.
        // An overlapping writer can change Page metadata after that dispatch;
        // concurrent profiles conservatively retain unrelated native uncertainty.
        if let Target::Context(context) = pending.target
            && self.shared.profile.writers == 1
        {
            for entry in &mut state.entries {
                if entry.target.context() == context {
                    entry.indeterminate = false;
                }
            }
        }
        prune_empty(&mut state);
        Ok(())
    }
}
impl Drop for HeaderUpdate {
    fn drop(&mut self) {
        // A poisoned Mutex already rejects all public reads/writes. Destructors
        // cannot return that error and must not attempt an async compensation.
        let Ok(mut state) = self.shared.state.lock() else {
            return;
        };
        let Some(index) = state
            .pending
            .iter()
            .position(|pending| pending.generation == self.generation)
        else {
            return;
        };
        if restore(&mut state, self.generation).is_err() {
            return;
        }
        let pending = state.pending.remove(index);
        if let Some(entry) = state
            .entries
            .iter_mut()
            .find(|entry| entry.target == pending.target)
        {
            entry.indeterminate = true;
        }
    }
}

fn restore(state: &mut State, generation: u64) -> Result<(), SettingsError> {
    let index = state
        .pending
        .iter()
        .position(|pending| pending.generation == generation)
        .ok_or(SettingsError::Superseded)?;
    let pending = &state.pending[index];
    if pending.restoring {
        return Ok(());
    }
    let target = pending.target;
    // Share the captured admitted value. Even an older failure must assign its
    // captured prior value, matching upstream rather than inventing latest-wins.
    let headers = pending
        .prior
        .as_ref()
        .map(|entry| entry.headers.clone())
        .unwrap_or(Headers::Absent);
    let entry = state
        .entries
        .iter_mut()
        .find(|entry| entry.target == target)
        .ok_or(SettingsError::Superseded)?;
    entry.headers = headers;
    entry.generation = generation;
    state.pending[index].restoring = true;
    Ok(())
}

fn prune_empty(state: &mut State) {
    state.entries.retain(|entry| {
        !entry.headers.is_empty()
            || entry.indeterminate
            || state
                .pending
                .iter()
                .any(|pending| pending.target == entry.target)
    });
}

#[cfg(test)]
mod tests;
