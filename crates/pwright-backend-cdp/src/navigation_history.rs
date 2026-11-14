use super::*;
use pwright_cap_history_navigation::{
    GoBack, GoBackParams, GoForward, GoForwardParams, HistoryNavigationErrorDetails,
    HistoryNavigationResult,
};

impl Service<GoBackParams> for CdpNavigationService {
    type Response = HistoryNavigationResult;
    type Error = OperationError<HistoryNavigationErrorDetails>;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, params: GoBackParams) -> Self::Future<'_> {
        self.inner.call(params)
    }
}

impl Service<GoForwardParams> for CdpNavigationService {
    type Response = HistoryNavigationResult;
    type Error = OperationError<HistoryNavigationErrorDetails>;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, params: GoForwardParams) -> Self::Future<'_> {
        self.inner.call(params)
    }
}

impl CommandCancellation<GoBack> for CdpNavigationService {
    type Future<'a> = Pin<
        Box<
            dyn Future<Output = Result<(), OperationError<HistoryNavigationErrorDetails>>>
                + Send
                + 'a,
        >,
    >;

    fn cancel(&self, context: OperationContext, reason: CancellationReason) -> Self::Future<'_> {
        <LeafCdpNavigationService as CommandCancellation<GoBack>>::cancel(
            &self.inner,
            context,
            reason,
        )
    }
}

impl CommandCancellation<GoForward> for CdpNavigationService {
    type Future<'a> = Pin<
        Box<
            dyn Future<Output = Result<(), OperationError<HistoryNavigationErrorDetails>>>
                + Send
                + 'a,
        >,
    >;

    fn cancel(&self, context: OperationContext, reason: CancellationReason) -> Self::Future<'_> {
        <LeafCdpNavigationService as CommandCancellation<GoForward>>::cancel(
            &self.inner,
            context,
            reason,
        )
    }
}
