//! Explicit, caller-admitted native close receipts; not foreign event dispatch.
use crate::{ChromiumBrowserContextCloseCommandService, ChromiumPageCloseCommandService};
use pwright_backend_cdp::{CdpTopologyEffect, CdpTopologyUpdate};
use pwright_cap_context_lifecycle::BrowserContextClose;
use pwright_cap_page_lifecycle::PageClose;
use pwright_core::{Command, CommandResult, ObjectId, OperationError, Service};
use std::{convert::Infallible, future::Future, pin::Pin, sync::Arc};

mod attachment;
pub use attachment::{ObservedAttachedContextCloseService, ObservedAttachedPageCloseService};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChromiumCloseTarget {
    Page(ObjectId),
    BrowserContext(ObjectId),
}

/// Borrowed effects from the exact prepared topology transaction. Not yet a
/// successful close: preparation can fail, be cancelled or be rolled back.
#[derive(Clone, Copy)]
pub struct ChromiumClosePlan<'a> {
    target: ChromiumCloseTarget,
    effects: &'a [CdpTopologyEffect],
}

impl<'a> ChromiumClosePlan<'a> {
    /// Construct a borrowed plan from the composition's exact prepared effects.
    /// This is not evidence of native success or an ancestor-retirement receipt.
    pub const fn new(target: ChromiumCloseTarget, effects: &'a [CdpTopologyEffect]) -> Self {
        Self { target, effects }
    }
    pub fn target(self) -> ChromiumCloseTarget {
        self.target
    }

    pub fn effects(self) -> &'a [CdpTopologyEffect] {
        self.effects
    }
}

/// Selected implementations own all reservation/retention admission, including
/// the returned box, before native close starts. No queue, task or subscription
/// is installed here. Preparation runs without a topology mutex held, but with
/// native topology/runtime transaction reservations active: do not reenter a
/// lifecycle command or call JavaScript.
pub trait ChromiumCloseObserver: Send + Sync {
    fn prepare(
        &self,
        plan: ChromiumClosePlan<'_>,
    ) -> Result<Box<dyn ChromiumCloseReservation>, OperationError<()>>;

    /// Optional two-phase admission for concurrent native closes. Implementations
    /// derive the target's maximum publication capacity before native mutation,
    /// without retaining a frozen visibility view. Legacy observers opt out.
    fn reserve_capacity(
        &self,
        _target: ChromiumCloseTarget,
    ) -> Result<Box<dyn ChromiumCloseCapacity>, OperationError<()>> {
        Err(OperationError::new(
            pwright_core::FailureKind::UnsupportedCapability,
            "deferred close publication is not installed",
            None,
        ))
    }
}

/// Pre-admitted storage/groups only, not a native receipt. After native work,
/// the composition serializes a short latest-state prepare/commit phase. This
/// consumes existing capacity without allocating a new publication group; do
/// not await or permit another lifecycle commit between prepare and committed.
/// The caller owns teardown if preparation fails after native mutation.
pub trait ChromiumCloseCapacity: Send {
    fn prepare(
        self: Box<Self>,
        plan: ChromiumClosePlan<'_>,
    ) -> Result<Box<dyn ChromiumCloseReservation>, OperationError<()>>;
}

/// Drop releases an abandoned admission. A started continuation owns this guard
/// until the native transaction finishes. `committed` consumes it exactly once,
/// after topology, runtime, routes and selected network retirement have committed
/// and their locks have been released. It receives the real committed effects,
/// not a collection difference. No rollback is possible if it returns an error.
///
/// This is a native journal input, never JS delivery. The following driver
/// snapshot has not necessarily been projected yet. Retain only pre-admitted
/// metadata; do not allocate a new queue or retain an execution owner here.
pub trait ChromiumCloseReservation: Send {
    fn committed(self: Box<Self>, effects: &[CdpTopologyEffect]) -> Result<(), OperationError<()>>;
}

/// Opt-in wrapper for an exact raw Page or Context close service. The ordinary
/// services keep their existing layout and install no observer. Wrap this in
/// the existing Continuable*CloseCommandService to retain cancellation safety.
#[derive(Clone)]
pub struct ObservedChromiumCloseCommandService<Inner> {
    service: Inner,
    observer: Arc<dyn ChromiumCloseObserver>,
}

impl<Inner> ObservedChromiumCloseCommandService<Inner> {
    pub fn new(service: Inner, observer: Arc<dyn ChromiumCloseObserver>) -> Self {
        Self { service, observer }
    }
}

impl Service<Command<PageClose>>
    for ObservedChromiumCloseCommandService<ChromiumPageCloseCommandService>
{
    type Response = CommandResult<PageClose>;
    type Error = Infallible;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, command: Command<PageClose>) -> Self::Future<'_> {
        let (call_id, object_id, metadata, ()) = command.into_parts();
        Box::pin(async move {
            let outcome = self
                .service
                .close(object_id, metadata, Some(self.observer.as_ref()), None)
                .await;
            Ok(CommandResult::new(call_id, outcome))
        })
    }
}

impl Service<Command<BrowserContextClose>>
    for ObservedChromiumCloseCommandService<ChromiumBrowserContextCloseCommandService>
{
    type Response = CommandResult<BrowserContextClose>;
    type Error = Infallible;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, command: Command<BrowserContextClose>) -> Self::Future<'_> {
        let (call_id, object_id, metadata, ()) = command.into_parts();
        Box::pin(async move {
            let outcome = self
                .service
                .close(object_id, metadata, Some(self.observer.as_ref()))
                .await;
            Ok(CommandResult::new(call_id, outcome))
        })
    }
}

pub(crate) fn prepare(
    observer: Option<&dyn ChromiumCloseObserver>,
    target: ChromiumCloseTarget,
    update: &CdpTopologyUpdate,
) -> Result<Option<Box<dyn ChromiumCloseReservation>>, OperationError<()>> {
    observer
        .map(|observer| {
            observer.prepare(ChromiumClosePlan {
                target,
                effects: update.effects(),
            })
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct LegacyObserver;

    impl ChromiumCloseObserver for LegacyObserver {
        fn prepare(
            &self,
            _plan: ChromiumClosePlan<'_>,
        ) -> Result<Box<dyn ChromiumCloseReservation>, OperationError<()>> {
            panic!("deferred admission must not call immediate preparation")
        }
    }

    #[test]
    fn legacy_observer_explicitly_opts_out_of_deferred_capacity() {
        // Arrange
        let observer = LegacyObserver;
        // Act
        let result = observer.reserve_capacity(ChromiumCloseTarget::Page(ObjectId::new(1)));
        // Assert
        assert!(
            matches!(result, Err(error) if error.kind() == pwright_core::FailureKind::UnsupportedCapability)
        );
    }
}
