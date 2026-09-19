//! Opt-in immutable header settings and allocation-admitted merge policy.
//! Native fanout, transactional installation, FFI values and routing are ports
//! of the embedding composition, not dependencies of this value model.
use pwright_core::{
    ProductionResourceError, ProductionResourceLedger, ResourceClass, ResourceError, ResourceLease,
    ResourceLedger, ResourceRequest, ResourceRequirements, ResourceScope,
};
use std::{
    fmt,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

pub mod settings;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct HeaderRef<'a> {
    pub name: &'a str,
    pub value: &'a str,
}
impl fmt::Debug for HeaderRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HeaderRef")
            .field("name_bytes", &self.name.len())
            .field("value_bytes", &self.value.len())
            .finish()
    }
}

#[derive(Debug)]
pub enum HeaderError {
    InvalidLimits,
    CountLimit,
    ByteLimit,
    OwnerLimit,
    ReferrerConflict,
    Budget(ResourceError),
    Plan(ProductionResourceError),
}
impl fmt::Display for HeaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimits => f.write_str("invalid or overflowing header profile"),
            Self::CountLimit => f.write_str("extra header count exceeds profile"),
            Self::ByteLimit => f.write_str("extra header bytes exceed profile"),
            Self::OwnerLimit => f.write_str("extra header owner capacity exhausted"),
            Self::ReferrerConflict => {
                f.write_str("\"referer\" is already specified as extra HTTP header")
            }
            Self::Budget(error) => error.fmt(f),
            Self::Plan(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for HeaderError {}

/// Native HTTP syntax rejection is distinct from desired-state admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeaderSyntaxError {
    NonAsciiName,
    InvalidName,
    InvalidValue,
}
impl fmt::Display for HeaderSyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NonAsciiName => "HTTP header name is not ASCII",
            Self::InvalidName => "Invalid header name",
            Self::InvalidValue => "Invalid header value",
        })
    }
}
impl std::error::Error for HeaderSyntaxError {}

#[derive(Clone, Copy, Debug)]
pub struct HeaderProfile {
    scope: ResourceScope,
    owners: usize,
    headers: usize,
    bytes: usize,
    owner_bytes: u64,
}
impl HeaderProfile {
    pub fn new(
        scope: ResourceScope,
        owners: usize,
        headers: usize,
        bytes: usize,
    ) -> Result<Self, HeaderError> {
        if owners == 0 || headers == 0 || bytes == 0 {
            return Err(HeaderError::InvalidLimits);
        }
        // One owned array plus a borrowed merge-planning array. Inputs are
        // caller-owned; string copies, owner/Arc metadata and lease request
        // storage are covered before allocating. Allocator padding is external.
        let owner_bytes = headers
            .checked_mul(std::mem::size_of::<Entry>() + std::mem::size_of::<HeaderRef<'static>>())
            .and_then(|n| n.checked_add(bytes))
            .and_then(|n| {
                n.checked_add(
                    std::mem::size_of::<Snapshot>() + 2 * std::mem::size_of::<usize>() + 512,
                )
            })
            .and_then(|n| u64::try_from(n).ok())
            .ok_or(HeaderError::InvalidLimits)?;
        let profile = Self {
            scope,
            owners,
            headers,
            bytes,
            owner_bytes,
        };
        profile.requirements()?;
        Ok(profile)
    }
    pub fn requirements(self) -> Result<ResourceRequirements, HeaderError> {
        let owners = u64::try_from(self.owners).map_err(|_| HeaderError::InvalidLimits)?;
        let bytes = self
            .owner_bytes
            .checked_mul(owners)
            .and_then(|n| n.checked_add(pool_bytes()))
            .ok_or(HeaderError::InvalidLimits)?;
        let leases = self
            .owners
            .checked_add(1)
            .ok_or(HeaderError::InvalidLimits)?;
        Ok(ResourceRequirements::none()
            .with_class(ResourceClass::LiveObject, owners, owners)
            .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
            .with_metadata(1, leases))
    }
}

struct Pool {
    profile: HeaderProfile,
    resources: ResourceLedger,
    active: AtomicUsize,
    _lease: ResourceLease,
}
fn pool_bytes() -> u64 {
    (std::mem::size_of::<Pool>() + 2 * std::mem::size_of::<usize>() + 256) as u64
}
struct Permit {
    pool: Arc<Pool>,
    _lease: ResourceLease,
}
impl Drop for Permit {
    fn drop(&mut self) {
        self.pool.active.fetch_sub(1, Ordering::AcqRel);
    }
}
#[derive(Clone)]
pub struct HeaderPool(Arc<Pool>);
impl HeaderPool {
    pub fn new(
        profile: HeaderProfile,
        ledger: &ProductionResourceLedger,
    ) -> Result<Self, HeaderError> {
        let resources = ledger
            .claim(profile.requirements()?)
            .map_err(HeaderError::Plan)?;
        let lease = resources
            .reserve(
                profile.scope,
                ResourceClass::BackendStateBytes,
                pool_bytes(),
            )
            .map_err(HeaderError::Budget)?;
        Ok(Self(Arc::new(Pool {
            profile,
            resources,
            active: AtomicUsize::new(0),
            _lease: lease,
        })))
    }
    pub fn active_owners(&self) -> usize {
        self.0.active.load(Ordering::Acquire)
    }
    fn admit(&self) -> Result<Permit, HeaderError> {
        let profile = self.0.profile;
        self.0
            .active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < profile.owners).then_some(n + 1)
            })
            .map_err(|_| HeaderError::OwnerLimit)?;
        let lease = self.0.resources.reserve_many(
            profile.scope,
            &[
                ResourceRequest::new(ResourceClass::LiveObject, 1),
                ResourceRequest::new(ResourceClass::BackendStateBytes, profile.owner_bytes),
            ],
        );
        match lease {
            Ok(lease) => Ok(Permit {
                pool: self.0.clone(),
                _lease: lease,
            }),
            Err(error) => {
                self.0.active.fetch_sub(1, Ordering::AcqRel);
                Err(HeaderError::Budget(error))
            }
        }
    }
    fn copy(&self, input: &[HeaderRef<'_>]) -> Result<Arc<Snapshot>, HeaderError> {
        validate_bounds(input, self.0.profile)?;
        let permit = self.admit()?;
        Ok(materialize(input, permit))
    }
    pub fn context(&self, input: &[HeaderRef<'_>]) -> Result<ContextHeaders, HeaderError> {
        self.copy(input).map(ContextHeaders)
    }
    pub fn page(&self, input: &[HeaderRef<'_>]) -> Result<PageHeaders, HeaderError> {
        self.copy(input).map(PageHeaders)
    }
    pub fn merge(
        &self,
        context: Option<&ContextHeaders>,
        page: Option<&PageHeaders>,
    ) -> Result<EffectiveHeaders, HeaderError> {
        let context = context.map(|value| &value.0.entries[..]).unwrap_or(&[]);
        let page = page.map(|value| &value.0.entries[..]).unwrap_or(&[]);
        if context.len() > self.0.profile.headers || page.len() > self.0.profile.headers {
            return Err(HeaderError::CountLimit);
        }
        let permit = self.admit()?;
        let mut selected: Vec<HeaderRef<'_>> = Vec::with_capacity(self.0.profile.headers);
        for entry in context.iter().chain(page) {
            let next = entry.borrowed();
            if let Some(prior) = selected
                .iter_mut()
                .find(|prior| prior.name.eq_ignore_ascii_case(next.name))
            {
                // Overwrite the value and original spelling, not its position.
                *prior = next;
            } else {
                if selected.len() == self.0.profile.headers {
                    return Err(HeaderError::CountLimit);
                }
                selected.push(next);
            }
        }
        // Validate final bytes, not an intermediate union that a later Page
        // replacement can shrink. All planning so far is borrowed and admitted.
        validate_bounds(&selected, self.0.profile)?;
        Ok(EffectiveHeaders(materialize(&selected, permit)))
    }
}

struct Entry {
    name: Box<str>,
    value: Box<str>,
}
impl Entry {
    fn borrowed(&self) -> HeaderRef<'_> {
        HeaderRef {
            name: &self.name,
            value: &self.value,
        }
    }
}
struct Snapshot {
    entries: Vec<Entry>,
    _permit: Permit,
}
impl fmt::Debug for Snapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HeaderSnapshot")
            .field("count", &self.entries.len())
            .finish()
    }
}
fn materialize(input: &[HeaderRef<'_>], permit: Permit) -> Arc<Snapshot> {
    let mut entries = Vec::with_capacity(input.len());
    for entry in input {
        entries.push(Entry {
            name: entry.name.into(),
            value: entry.value.into(),
        });
    }
    Arc::new(Snapshot {
        entries,
        _permit: permit,
    })
}
// Desired settings are strings, not prevalidated native HTTP headers. A later
// Page value can mask an invalid Context value, and a Context with no Page can
// retain invalid syntax until native installation. Only allocation bounds apply.
fn validate_bounds(input: &[HeaderRef<'_>], profile: HeaderProfile) -> Result<(), HeaderError> {
    if input.len() > profile.headers {
        return Err(HeaderError::CountLimit);
    }
    let mut bytes = 0usize;
    for entry in input {
        bytes = bytes
            .checked_add(entry.name.len())
            .and_then(|n| n.checked_add(entry.value.len()))
            .ok_or(HeaderError::ByteLimit)?;
        if bytes > profile.bytes {
            return Err(HeaderError::ByteLimit);
        }
    }
    Ok(())
}

fn validate_http_syntax<'a>(
    input: impl Iterator<Item = HeaderRef<'a>>,
) -> Result<(), HeaderSyntaxError> {
    for entry in input {
        if !entry.name.is_ascii() {
            return Err(HeaderSyntaxError::NonAsciiName);
        }
        if entry.name.is_empty()
            || !entry
                .name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
        {
            return Err(HeaderSyntaxError::InvalidName);
        }
        if entry
            .value
            .bytes()
            .any(|b| (b < 32 && b != b'\t') || b == 127)
        {
            return Err(HeaderSyntaxError::InvalidValue);
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct ContextHeaders(Arc<Snapshot>);
#[derive(Clone, Debug)]
pub struct PageHeaders(Arc<Snapshot>);
#[derive(Clone, Debug)]
pub struct EffectiveHeaders(Arc<Snapshot>);
impl ContextHeaders {
    pub fn iter(&self) -> impl ExactSizeIterator<Item = HeaderRef<'_>> {
        self.0.entries.iter().map(Entry::borrowed)
    }
}
impl PageHeaders {
    pub fn iter(&self) -> impl ExactSizeIterator<Item = HeaderRef<'_>> {
        self.0.entries.iter().map(Entry::borrowed)
    }
    /// Deliberately reads the first original Page header, not merged Context
    /// or last-wins effective headers. The option and result are borrowed.
    pub fn navigation_referrer<'a>(
        &'a self,
        option: Option<&'a str>,
    ) -> Result<Option<&'a str>, HeaderError> {
        let header = self
            .iter()
            .find(|entry| entry.name.eq_ignore_ascii_case("referer"))
            .map(|entry| entry.value);
        match (header, option) {
            (Some(header), Some(option)) if header != option => Err(HeaderError::ReferrerConflict),
            (_, Some(option)) => Ok(Some(option)),
            (header, None) => Ok(header),
        }
    }
}
impl EffectiveHeaders {
    pub fn iter(&self) -> impl ExactSizeIterator<Item = HeaderRef<'_>> {
        self.0.entries.iter().map(Entry::borrowed)
    }
    /// Check final HTTP syntax at the native installation boundary, after
    /// Context/Page precedence and duplicate replacement. This borrowed check
    /// allocates nothing and is not a native acknowledgement.
    pub fn validate_http_syntax(&self) -> Result<(), HeaderSyntaxError> {
        validate_http_syntax(self.iter())
    }
}

#[cfg(test)]
mod tests;
