use pwright_cap_page_lifecycle::PageLifecycleErrorDetails;
use pwright_core::OperationError;
use std::{future::Future, pin::Pin};

pub type PageCloseFuture =
    Pin<Box<dyn Future<Output = Result<(), OperationError<PageLifecycleErrorDetails>>> + Send>>;

/// Opt-in native confirmation policy. Preparation must admit every retained
/// owner, identity, subscription and future before a target-close side effect.
/// The policy binds its command client and receipt source to the same connection.
pub trait CdpPageCloseConfirmation: Send + Sync {
    fn prepare(
        &self,
        target: &str,
    ) -> Result<Box<dyn CdpPreparedPageClose>, OperationError<PageLifecycleErrorDetails>>;
}

/// Owned by the existing continuation after execution begins. Preparation/boxing
/// failure sends no target-close command; dropping an unstarted owner releases
/// its admission. Implementations must not infer target death from session loss.
pub trait CdpPreparedPageClose: Send {
    fn start(self: Box<Self>)
    -> Result<PageCloseFuture, OperationError<PageLifecycleErrorDetails>>;
}
