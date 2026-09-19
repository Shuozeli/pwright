//! Optional navigation-name storage. Leases outlive the book with retained Frames.

use super::*;

const NAME_SCOPE: ResourceScope = ResourceScope::extension(121, 3);

#[derive(Clone, Copy)]
pub(crate) struct FrameNameLimits {
    pub name_bytes: usize,
    pub total_bytes: u64,
}

pub(super) struct NameResources {
    resources: ResourceLedger,
    limits: FrameNameLimits,
}

pub(super) struct BudgetedName {
    pub value: Box<str>,
    _bytes: ResourceLease,
}

impl NameResources {
    pub fn new(metadata: FrameMetadataLimits, limits: FrameNameLimits) -> Result<Self, HostError> {
        // Each retained entry can have distinct native/published revisions;
        // each queued publication can retain one additional name revision.
        let leases = metadata
            .entries
            .checked_mul(2)
            .and_then(|entries| entries.checked_add(metadata.publications))
            .ok_or_else(|| {
                HostError::ResourceExhausted("Frame name lease capacity overflow".into())
            })?;
        let requirements = ResourceRequirements::none()
            .with_class(
                ResourceClass::BackendStateBytes,
                limits.total_bytes,
                limits.total_bytes,
            )
            .with_metadata(1, leases);
        let resource_limits = ResourceLimits::deny_all(1, leases).with_class(
            ResourceClass::BackendStateBytes,
            limits.total_bytes,
            limits.total_bytes,
        );
        let resources = ProductionResourceLedger::new(resource_limits, requirements)
            .and_then(|ledger| ledger.claim(requirements))
            .map_err(HostError::backend)?;
        Ok(Self { resources, limits })
    }

    pub fn allocate(&self, value: &str) -> Result<Option<Arc<BudgetedName>>, HostError> {
        if value.len() > self.limits.name_bytes {
            return Err(HostError::ResourceExhausted(
                "Frame name exceeds native metadata byte limit".into(),
            ));
        }
        if value.is_empty() {
            return Ok(None);
        }
        let bytes = value
            .len()
            .checked_add(std::mem::size_of::<BudgetedName>())
            .and_then(|n| n.checked_add(2 * std::mem::size_of::<usize>()))
            .ok_or_else(|| {
                HostError::ResourceExhausted("Frame name allocation size overflow".into())
            })?;
        let lease = self
            .resources
            .reserve(
                NAME_SCOPE,
                ResourceClass::BackendStateBytes,
                u64::try_from(bytes).map_err(HostError::backend)?,
            )
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        Ok(Some(Arc::new(BudgetedName {
            value: value.into(),
            _bytes: lease,
        })))
    }

    pub fn bytes(&self) -> Result<u32, HostError> {
        let metrics = self.resources.metrics().map_err(HostError::backend)?;
        u32::try_from(metrics.class(ResourceClass::BackendStateBytes).current)
            .map_err(HostError::backend)
    }
}

pub(super) fn value(name: &Option<Arc<BudgetedName>>) -> &str {
    name.as_ref().map_or("", |name| &name.value)
}
