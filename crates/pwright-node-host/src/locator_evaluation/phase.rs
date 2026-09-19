//! One deadline across queue/selection/adoption, none after the handoff.
//! Target/transport lifecycle remains in the controlled command throughout.
use crate::HostError;
use std::{
    future::{Future, poll_fn},
    pin::pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::Poll,
};

pub(super) async fn run<T>(
    operation: impl Future<Output = Result<T, HostError>>,
    preparing: Arc<AtomicBool>,
    deadline: impl Future<Output = ()>,
    stopped: impl Future<Output = ()>,
    timeout_ms: u32,
) -> Result<T, HostError> {
    let mut operation = pin!(operation);
    let mut deadline = pin!(deadline);
    let mut stopped = pin!(stopped);
    poll_fn(|cx| {
        if stopped.as_mut().poll(cx).is_ready() {
            return Poll::Ready(Err(HostError::TargetClosed));
        }
        if preparing.load(Ordering::Acquire) && deadline.as_mut().poll(cx).is_ready() {
            return Poll::Ready(Err(HostError::Timeout(timeout_ms)));
        }
        operation.as_mut().poll(cx)
    })
    .await
}
