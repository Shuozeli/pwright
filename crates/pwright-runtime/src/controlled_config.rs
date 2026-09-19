use std::num::NonZeroUsize;

use crate::RuntimeLimits;

/// Explicit scheduling selection for a controlled runtime. Existing constructors
/// accepting `RuntimeLimits` retain one active command per target. The connection
/// and pending-call budgets remain authoritative with either selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ControlledRuntimeConfig {
    limits: RuntimeLimits,
    target_concurrency: NonZeroUsize,
}

impl ControlledRuntimeConfig {
    pub const fn new(limits: RuntimeLimits) -> Self {
        Self {
            limits,
            target_concurrency: NonZeroUsize::MIN,
        }
    }

    pub const fn with_target_concurrency(mut self, maximum: NonZeroUsize) -> Self {
        self.target_concurrency = maximum;
        self
    }

    pub const fn limits(self) -> RuntimeLimits {
        self.limits
    }
    pub const fn target_concurrency(self) -> NonZeroUsize {
        self.target_concurrency
    }
}

impl From<RuntimeLimits> for ControlledRuntimeConfig {
    fn from(limits: RuntimeLimits) -> Self {
        Self::new(limits)
    }
}
