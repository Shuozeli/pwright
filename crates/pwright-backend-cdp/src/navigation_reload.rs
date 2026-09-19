use super::*;
use pwright_cap_navigation::{Reload, ReloadParams};

impl Service<ReloadParams> for CdpNavigationService {
    type Response = GotoResult;
    type Error = OperationError<NavigationErrorDetails>;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, params: ReloadParams) -> Self::Future<'_> {
        self.inner.call(params)
    }
}

impl CommandCancellation<Reload> for CdpNavigationService {
    type Future<'a> = Pin<
        Box<dyn Future<Output = Result<(), OperationError<NavigationErrorDetails>>> + Send + 'a>,
    >;

    fn cancel(&self, context: OperationContext, reason: CancellationReason) -> Self::Future<'_> {
        <LeafCdpNavigationService as CommandCancellation<Reload>>::cancel(
            &self.inner,
            context,
            reason,
        )
    }
}
