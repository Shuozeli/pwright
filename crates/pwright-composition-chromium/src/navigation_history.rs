use super::*;
use pwright_cap_history_navigation::{
    GoBackParams, GoForwardParams, HistoryNavigationErrorDetails, HistoryNavigationResult,
};

pub(super) fn history_navigation_route_state_error(
    error: CdpPageRouteError,
) -> OperationError<HistoryNavigationErrorDetails> {
    OperationError::new(
        FailureKind::Cleanup,
        error.to_string(),
        Some(HistoryNavigationErrorDetails::Backend(
            "page route state is unavailable".to_owned(),
        )),
    )
}

fn missing_history_navigation_route(
    object_id: ObjectId,
) -> OperationError<HistoryNavigationErrorDetails> {
    OperationError::new(
        FailureKind::TargetClosed,
        format!("page `{object_id}` has no active CDP session"),
        Some(HistoryNavigationErrorDetails::Backend(
            "target session is unavailable".to_owned(),
        )),
    )
}

impl TargetedService<GoBackParams> for CdpPageServices {
    type Response = HistoryNavigationResult;
    type Error = OperationError<HistoryNavigationErrorDetails>;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call_for(&self, object_id: ObjectId, params: GoBackParams) -> Self::Future<'_> {
        let service = self
            .route(object_id)
            .map_err(history_navigation_route_state_error)
            .map(|entry| entry.and_then(|entry| entry.navigation.clone()));
        Box::pin(async move {
            let service = service?.ok_or_else(|| missing_history_navigation_route(object_id))?;
            service.call(params).await
        })
    }
}

impl TargetedService<GoForwardParams> for CdpPageServices {
    type Response = HistoryNavigationResult;
    type Error = OperationError<HistoryNavigationErrorDetails>;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call_for(&self, object_id: ObjectId, params: GoForwardParams) -> Self::Future<'_> {
        let service = self
            .route(object_id)
            .map_err(history_navigation_route_state_error)
            .map(|entry| entry.and_then(|entry| entry.navigation.clone()));
        Box::pin(async move {
            let service = service?.ok_or_else(|| missing_history_navigation_route(object_id))?;
            service.call(params).await
        })
    }
}
