//! Long-lived selector waits share one environment-wide fraction of Command
//! admission. A second Session must not consume the slots left for unblockers.

use std::sync::Arc;

use pwright_node_host::HostError;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

pub(crate) const COMMAND_CAPACITY: usize = 32;
pub(crate) const RESERVED_COMMANDS: usize = 8;
pub(crate) const MAX_LONG_WAITS: usize = COMMAND_CAPACITY - RESERVED_COMMANDS;
const _: () = assert!(MAX_LONG_WAITS == 24 && RESERVED_COMMANDS > 0);

pub(crate) type WaitPermit = OwnedSemaphorePermit;

#[derive(Clone)]
pub(crate) struct WaitAdmissions(Arc<Semaphore>);

impl WaitAdmissions {
    pub(crate) fn new() -> Self {
        Self(Arc::new(Semaphore::new(MAX_LONG_WAITS)))
    }

    /// Reject immediately before FFI Command admission; never queue a wait
    /// behind an operation that may require another Command to unblock it.
    pub(crate) fn admit(&self) -> Result<WaitPermit, HostError> {
        Arc::clone(&self.0).try_acquire_owned().map_err(|_| {
            HostError::ResourceExhausted(format!(
                "FFI long wait capacity exceeded ({MAX_LONG_WAITS}); {RESERVED_COMMANDS} Command slots reserved for unblockers"
            ))
        })
    }
}
