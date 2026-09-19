use super::*;
use pwright_cap_navigation::Reload;

impl Service<Command<Reload>> for ChromiumNavigationPage {
    type Response = CommandResult<Reload>;
    type Error = Infallible;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, command: Command<Reload>) -> Self::Future<'_> {
        let (call_id, object_id, _metadata, params) = command.into_parts();
        let route = (object_id == self.page_id)
            .then_some(())
            .ok_or_else(|| self.wrong_target(object_id));
        Box::pin(async move {
            let outcome = match route {
                Ok(()) => self.backend.call(params).await,
                Err(error) => Err(error),
            };
            Ok(CommandResult::new(call_id, outcome))
        })
    }
}
