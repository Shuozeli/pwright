use super::*;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

/// Opt-in, single-consumer receipt for native evaluation retirement.
///
/// Implementors MUST return Ready only after authoritative retirement of this
/// exact client's native session/epoch and unique execution context, including
/// outstanding inspector evaluation callbacks and any late result wrappers.
/// Admission and native dispatch must be fenced before that acknowledgement.
/// A cancelled user call, group release, generic detach event, transport loss,
/// timeout or matching error text is not such evidence. User JS need not stop.
///
/// Pending implementations register the supplied waker without lost wakeups.
/// The composing caller budgets all retained receipt state through cleanup.
/// This receipt only ends the native wait: source-group cleanup independently
/// requires a release acknowledgement or its own authoritative proof.
pub trait NativeEvaluationRetirement: Send {
    fn poll_retired(
        &mut self,
        client: &Arc<dyn CdpClient>,
        unique_context_id: &str,
        cx: &mut Context<'_>,
    ) -> Poll<()>;
}

pub(super) async fn wait<F>(
    native: F,
    mut retirement: Option<&mut (dyn NativeEvaluationRetirement + 'static)>,
    client: &Arc<dyn CdpClient>,
    unique_context_id: &str,
) -> Outcome<EvaluationValue>
where
    F: Future<Output = Result<EvaluationValue, OperationError<HandleEvaluationErrorDetails>>>,
{
    let mut native = std::pin::pin!(native);
    std::future::poll_fn(|cx| {
        if retirement
            .as_mut()
            .is_some_and(|proof| proof.poll_retired(client, unique_context_id, cx).is_ready())
        {
            return Poll::Ready(Err(ElementEvaluationError::NativeRetired));
        }
        Pin::as_mut(&mut native).poll(cx).map_err(Into::into)
    })
    .await
}
