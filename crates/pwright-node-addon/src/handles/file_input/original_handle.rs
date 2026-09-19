//! Inert identity of the actual native wrapper supplied at preparation ingress.

use super::super::{NativeHandle, State};
#[cfg(feature = "retained-target-marking")]
use pwright_node_host::{HostError, handles::PendingHandle};
use std::sync::{Arc, Mutex, Weak};

/// This is not a native-use pin: capture/clone do not lock, upgrade, read Live
/// state or reject disposal. The eventual native phase must revalidate before
/// acquiring an owner. An unrelated JS receiver cannot replace this binding.
/// Stale Weak backing may outlive active admission; no heap reclamation or
/// arbitrary retained-wrapper byte bound is implied.
pub(super) struct OriginalHandle(Weak<Mutex<State>>);

impl OriginalHandle {
    pub(super) fn capture(handle: &NativeHandle) -> Self {
        Self(Arc::downgrade(&handle.state))
    }

    /// Upgrade only at the admitted native boundary, after observable language
    /// preparation. The returned descriptor is not a pin against concurrent
    /// explicit disposal or world destruction; host dispatch still revalidates.
    #[cfg(feature = "retained-target-marking")]
    pub(super) fn upgrade(&self) -> Result<Arc<PendingHandle>, HostError> {
        let original = self.0.upgrade().ok_or(HostError::TargetClosed)?;
        let result = {
            let state = original.lock().map_err(|_| {
                HostError::Backend("Original Handle state mutex is poisoned".into())
            })?;
            match &*state {
                State::Live(handle) => Ok(Arc::clone(handle)),
                State::Disposed(_) => Err(HostError::Backend("JSHandle is disposed".into())),
            }
        };
        // Neither the mutex guard nor the wrapper-state Arc escapes this call.
        result
    }
}

impl Clone for OriginalHandle {
    fn clone(&self) -> Self {
        Self(Weak::clone(&self.0))
    }
}

const _: () =
    assert!(std::mem::size_of::<OriginalHandle>() == std::mem::size_of::<Weak<Mutex<State>>>());

#[cfg(all(test, feature = "retained-target-marking"))]
mod tests {
    use super::*;

    #[test]
    fn missing_original_wrapper_rejects_late_upgrade_as_target_closed() {
        // Arrange: no fabricated PendingHandle or native release receipt.
        let original = OriginalHandle(Weak::new());

        // Act.
        let outcome = original.upgrade();

        // Assert.
        assert!(matches!(outcome, Err(HostError::TargetClosed)));
    }
}
