//! Optional per-transaction native keydown interception; no Page registry/task.
use pwright_cdp::{CdpClient, connection::Result};
use std::{future::Future, pin::Pin, sync::Arc};

pub type KeyDownInterceptionFuture = Pin<Box<dyn Future<Output = Result<bool>> + Send>>;

/// Called after key resolution and logical pressed-key/modifier updates, before
/// native keydown. None is an allocation-free pass-through; Some returns true
/// only when native keydown should be suppressed. Keyup and insertText bypass it.
///
/// Construction must perform no native IO. The provider admits its own storage
/// and retains started native owners independently if this future is dropped;
/// the embedding Page/host must drive that provider's retirement before teardown.
pub trait CdpKeyDownInterceptor: Send + Sync {
    fn intercept(
        self: Arc<Self>,
        client: Arc<dyn CdpClient>,
        code: &'static str,
    ) -> Option<KeyDownInterceptionFuture>;
}
