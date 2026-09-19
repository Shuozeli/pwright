//! Seal caller admission without invoking Rust wakeups under the session lock.

use crate::HostError;
use std::sync::Mutex;
use tokio_util::{sync::CancellationToken, task::TaskTracker};

/// All admissions must test `stop` and publish their work token while holding
/// `state`. Cancellation happens outside that lock, then the lock acts as a
/// barrier for an admission that observed the earlier connected state. Only
/// after that barrier can closing the tracker observe a stable empty set.
///
/// `take` is pure internal bookkeeping: it may seal/take weak revocation
/// entries, but must not wake, drop native owners, or call foreign code. The
/// returned batch must be revoked outside locks before awaiting caller/native
/// drains. Cancellation and tracker closure can both synchronously reenter.
pub(crate) fn cancel_then_seal<T, R>(
    state: &Mutex<T>,
    stop: &CancellationToken,
    operations: &TaskTracker,
    take: impl FnOnce(&T) -> R,
) -> Result<R, HostError> {
    stop.cancel();
    let sealed = {
        let state = state.lock().map_err(HostError::backend)?;
        take(&state)
    };
    operations.close();
    Ok(sealed)
}

#[cfg(test)]
mod tests;
