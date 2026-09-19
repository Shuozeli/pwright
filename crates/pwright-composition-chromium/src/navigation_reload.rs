use super::*;
use pwright_cap_navigation::ReloadParams;

impl TargetedService<ReloadParams> for CdpPageServices {
    type Response = GotoResult;
    type Error = OperationError<NavigationErrorDetails>;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call_for(&self, object_id: ObjectId, params: ReloadParams) -> Self::Future<'_> {
        let service = self
            .route(object_id)
            .map_err(navigation_route_state_error)
            .map(|entry| entry.and_then(|entry| entry.navigation.clone()));
        Box::pin(async move {
            let service = service?.ok_or_else(|| missing_navigation_route(object_id))?;
            service.call(params).await
        })
    }
}
