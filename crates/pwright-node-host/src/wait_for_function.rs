//! Optional bounded wait-for-function orchestration over Rust-owned Handle evaluation.
//!
//! The predicate executes in its Frame, but Rust owns admission, deadline,
//! cancellation and the exact truthy Handle returned to the language binding.

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;

use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequirements,
    ResourceScope,
};
use tokio_util::sync::CancellationToken;

use crate::handles::{
    EvaluateHandleParams, HandleEvaluateParams, OwnedHandleArgument, PendingHandle,
};
use crate::{HostError, HostSession, evaluation::EvaluationValue};

const MAX_OPERATIONS: u64 = 8;
const OPERATION_BYTES: u64 = 64 * 1024;
const OPERATION_SCOPE: ResourceScope = ResourceScope::extension(139, 1);
const SLOT_SCOPE: ResourceScope = ResourceScope::extension(139, 2);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Polling {
    AnimationFrame,
    Interval(u32),
}

struct Slots {
    active: AtomicU64,
    resources: ProductionResourceLedger,
    _lease: ResourceLease,
}

struct OperationPermit {
    owner: Arc<Slots>,
    _storage: ResourceLease,
}

impl Drop for OperationPermit {
    fn drop(&mut self) {
        self.owner.active.fetch_sub(1, Ordering::AcqRel);
    }
}

impl Slots {
    fn new(resources: &ProductionResourceLedger) -> Result<Arc<Self>, HostError> {
        let plan = slot_requirements();
        let lease = resources
            .claim(plan)
            .map_err(HostError::backend)?
            .reserve(
                SLOT_SCOPE,
                ResourceClass::BackendStateBytes,
                plan.class(ResourceClass::BackendStateBytes).global,
            )
            .map_err(HostError::backend)?;
        Ok(Arc::new(Self {
            active: AtomicU64::new(0),
            resources: resources.clone(),
            _lease: lease,
        }))
    }

    fn admit(self: &Arc<Self>) -> Result<OperationPermit, HostError> {
        self.active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
                (active < MAX_OPERATIONS).then_some(active + 1)
            })
            .map_err(|_| {
                HostError::ResourceExhausted(format!(
                    "wait_for_function capacity exceeded ({MAX_OPERATIONS})"
                ))
            })?;
        let storage = self
            .resources
            .claim(operation_requirements())
            .map_err(HostError::backend)
            .and_then(|ledger| {
                ledger
                    .reserve(
                        OPERATION_SCOPE,
                        ResourceClass::BackendStateBytes,
                        OPERATION_BYTES,
                    )
                    .map_err(HostError::backend)
            });
        match storage {
            Ok(storage) => Ok(OperationPermit {
                owner: self.clone(),
                _storage: storage,
            }),
            Err(error) => {
                self.active.fetch_sub(1, Ordering::AcqRel);
                Err(error)
            }
        }
    }
}

pub(crate) struct Installation {
    slots: Arc<Slots>,
}

const fn slot_requirements() -> ResourceRequirements {
    let bytes = (std::mem::size_of::<Slots>() + 2 * std::mem::size_of::<usize>() + 256) as u64;
    ResourceRequirements::none()
        .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
        .with_metadata(1, 1)
}

const fn operation_requirements() -> ResourceRequirements {
    ResourceRequirements::none()
        .with_class(
            ResourceClass::BackendStateBytes,
            OPERATION_BYTES * MAX_OPERATIONS,
            OPERATION_BYTES * MAX_OPERATIONS,
        )
        .with_metadata(1, MAX_OPERATIONS as usize)
}

pub(crate) fn requirements() -> ResourceRequirements {
    operation_requirements().combine_concurrent(slot_requirements())
}

pub(crate) fn install(resources: &ProductionResourceLedger) -> Result<Installation, HostError> {
    Ok(Installation {
        slots: Slots::new(resources)?,
    })
}

impl HostSession {
    pub async fn frame_wait_for_function(
        &self,
        frame: &str,
        source: String,
        is_function: bool,
        argument: EvaluationValue,
        polling: Polling,
        timeout_ms: u32,
    ) -> Result<PendingHandle, HostError> {
        self.wait_for_function_operation(timeout_ms, |deadline| async move {
            loop {
                let params = if is_function {
                    EvaluateHandleParams::function(source.clone(), argument.clone())
                } else {
                    EvaluateHandleParams::expression(source.clone())
                };
                let prepared = self.frame_prepare_handle(frame, params).await;
                if deadline.is_cancelled() {
                    if let Ok(handle) = prepared {
                        handle.request_dispose();
                    }
                    return Err(HostError::Timeout(timeout_ms));
                }
                match prepared {
                    Ok(handle) => {
                        let finished = self.finish_attempt(handle).await;
                        if deadline.is_cancelled() {
                            if let Ok(Some(handle)) = finished {
                                handle.request_dispose();
                            }
                            return Err(HostError::Timeout(timeout_ms));
                        }
                        match finished {
                            Ok(Some(handle)) => return Ok(handle),
                            Ok(None) => {}
                            Err(error) if context_was_replaced(&error) => {
                                if !self.frame_is_active(frame)? {
                                    return Err(terminal_context_loss(error));
                                }
                            }
                            Err(error) => return Err(error),
                        }
                    }
                    Err(error) if context_was_replaced(&error) => {
                        if !self.frame_is_active(frame)? {
                            return Err(terminal_context_loss(error));
                        }
                    }
                    Err(error) => return Err(error),
                }
                tokio::select! {
                    biased;
                    () = deadline.cancelled() => return Err(HostError::Timeout(timeout_ms)),
                    () = wait_poll(polling) => {}
                }
            }
        })
        .await
    }

    pub async fn frame_wait_for_function_owned(
        &self,
        frame: &str,
        source: String,
        is_function: bool,
        argument: OwnedHandleArgument,
        polling: Polling,
        timeout_ms: u32,
    ) -> Result<PendingHandle, HostError> {
        self.wait_for_function_operation(timeout_ms, |deadline| async move {
            loop {
                let prepared = self
                    .frame_prepare_handle_owned(
                        frame,
                        source.clone(),
                        is_function,
                        argument.clone(),
                    )
                    .await;
                if deadline.is_cancelled() {
                    if let Ok(handle) = prepared {
                        handle.request_dispose();
                    }
                    return Err(HostError::Timeout(timeout_ms));
                }
                match prepared {
                    Ok(handle) => {
                        let finished = self.finish_attempt(handle).await;
                        if deadline.is_cancelled() {
                            if let Ok(Some(handle)) = finished {
                                handle.request_dispose();
                            }
                            return Err(HostError::Timeout(timeout_ms));
                        }
                        match finished {
                            Ok(Some(handle)) => return Ok(handle),
                            Ok(None) => {}
                            Err(error) if context_was_replaced(&error) => {
                                if !self.frame_is_active(frame)? {
                                    return Err(terminal_context_loss(error));
                                }
                            }
                            Err(error) => return Err(error),
                        }
                    }
                    Err(error) if context_was_replaced(&error) => {
                        if !self.frame_is_active(frame)? {
                            return Err(terminal_context_loss(error));
                        }
                    }
                    Err(error) => return Err(error),
                }
                tokio::select! {
                    biased;
                    () = deadline.cancelled() => return Err(HostError::Timeout(timeout_ms)),
                    () = wait_poll(polling) => {}
                }
            }
        })
        .await
    }

    async fn finish_attempt(
        &self,
        handle: PendingHandle,
    ) -> Result<Option<PendingHandle>, HostError> {
        let truthy = self
            .handle_evaluate(
                &handle,
                HandleEvaluateParams::function_without_argument("value => !!value"),
            )
            .await;
        match truthy {
            Ok(EvaluationValue::Bool(true)) => Ok(Some(handle)),
            Ok(EvaluationValue::Bool(false)) => {
                handle.dispose().await?;
                Ok(None)
            }
            Ok(_) => {
                handle.request_dispose();
                Err(HostError::Backend(
                    "wait_for_function truthiness probe returned a non-boolean value".into(),
                ))
            }
            Err(error) => {
                handle.request_dispose();
                Err(error)
            }
        }
    }

    fn frame_is_active(&self, frame: &str) -> Result<bool, HostError> {
        let frame = frame
            .parse::<u64>()
            .ok()
            .filter(|value| *value != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Frame ID".into()))?;
        self.frame_metadata.is_active(frame)
    }

    async fn wait_for_function_operation<Start, F>(
        &self,
        timeout_ms: u32,
        start: Start,
    ) -> Result<PendingHandle, HostError>
    where
        Start: FnOnce(CancellationToken) -> F,
        F: std::future::Future<Output = Result<PendingHandle, HostError>>,
    {
        let _permit = {
            let state = self.state.lock().map_err(HostError::backend)?;
            state
                .as_ref()
                .ok_or(HostError::TargetClosed)?
                .wait_for_function
                .slots
                .admit()?
        };
        run_with_deadline(&self.stop, &self.disconnected, timeout_ms, start).await
    }
}

async fn run_with_deadline<T, Start, F>(
    stop: &CancellationToken,
    disconnected: &CancellationToken,
    timeout_ms: u32,
    start: Start,
) -> Result<T, HostError>
where
    Start: FnOnce(CancellationToken) -> F,
    F: std::future::Future<Output = Result<T, HostError>>,
{
    let deadline = CancellationToken::new();
    let mut operation = Box::pin(start(deadline.clone()));
    let outcome = tokio::select! {
        biased;
        () = stop.cancelled() => return Err(HostError::TargetClosed),
        () = disconnected.cancelled() => return Err(HostError::TargetClosed),
        () = wait_deadline(timeout_ms) => None,
        result = &mut operation => Some(result),
    };
    if let Some(result) = outcome {
        return result;
    }

    // Do not drop an admitted predicate evaluation at the public deadline.
    // Its browser command can already have produced observable side effects,
    // and dropping only transfers native completion to a cleanup continuation.
    // Signal the polling loop, then retain it until the current attempt and its
    // ordinary command/publication fence have settled. The loop checks this
    // signal before it can start another attempt.
    deadline.cancel();
    tokio::select! {
        biased;
        () = stop.cancelled() => Err(HostError::TargetClosed),
        () = disconnected.cancelled() => Err(HostError::TargetClosed),
        result = &mut operation => result,
    }
}

fn context_was_replaced(error: &HostError) -> bool {
    match error {
        HostError::TargetClosed => true,
        HostError::Backend(message) => {
            message.contains("uniqueContextId not found")
                || message.contains("Execution context was destroyed")
                || message.contains("Cannot find context with specified id")
        }
        _ => false,
    }
}

fn terminal_context_loss(error: HostError) -> HostError {
    match error {
        HostError::TargetClosed | HostError::Backend(_) => HostError::Backend(
            "Execution context was destroyed, most likely because of a navigation".into(),
        ),
        error => error,
    }
}

async fn wait_poll(polling: Polling) {
    let milliseconds = match polling {
        // Rust owns cancellation between attempts; 16 ms approximates one
        // foreground animation frame without leaving a browser Promise alive.
        Polling::AnimationFrame => 16,
        Polling::Interval(milliseconds) => milliseconds,
    };
    tokio::time::sleep(Duration::from_millis(u64::from(milliseconds))).await;
}

async fn wait_deadline(timeout_ms: u32) {
    if timeout_ms == 0 {
        std::future::pending().await
    } else {
        tokio::time::sleep(Duration::from_millis(u64::from(timeout_ms))).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_core::ResourceLimits;

    fn resources() -> ProductionResourceLedger {
        let plan = requirements();
        let mut limits = ResourceLimits::deny_all(plan.scopes, plan.leases);
        for class in ResourceClass::ALL {
            let bound = plan.class(class);
            limits = limits.with_class(class, bound.global, bound.per_scope);
        }
        ProductionResourceLedger::new(limits, plan).unwrap()
    }

    #[test]
    fn polling_choice_keeps_animation_frames_and_intervals_distinct() {
        // Arrange
        let animation = Polling::AnimationFrame;

        // Act
        let interval = Polling::Interval(25);

        // Assert
        assert_ne!(animation, interval);
        assert_eq!(interval, Polling::Interval(25));
    }

    #[test]
    fn operation_capacity_is_fixed_and_reclaimed_on_drop() {
        // Arrange
        let resources = resources();
        let slots = Slots::new(&resources).unwrap();
        let mut permits: Vec<_> = (0..MAX_OPERATIONS)
            .map(|_| slots.admit().unwrap())
            .collect();

        // Act
        let excess = slots.admit();
        drop(permits.pop());
        let reclaimed = slots.admit();

        // Assert
        assert!(matches!(excess, Err(HostError::ResourceExhausted(_))));
        assert!(reclaimed.is_ok());
    }

    #[test]
    fn replaced_context_diagnostics_are_retryable() {
        // Arrange
        let target_closed = HostError::TargetClosed;
        let unique_context = HostError::Backend("CDP error: uniqueContextId not found".into());
        let destroyed = HostError::Backend("Execution context was destroyed".into());
        let missing = HostError::Backend("Cannot find context with specified id".into());

        // Act
        let outcomes = [
            context_was_replaced(&target_closed),
            context_was_replaced(&unique_context),
            context_was_replaced(&destroyed),
            context_was_replaced(&missing),
        ];

        // Assert
        assert_eq!(outcomes, [true, true, true, true]);
    }

    #[test]
    fn unrelated_backend_failures_are_not_retried() {
        // Arrange
        let error = HostError::Backend("predicate exploded".into());

        // Act
        let retry = context_was_replaced(&error);

        // Assert
        assert!(!retry);
    }

    #[test]
    fn terminal_backend_context_loss_uses_playwright_diagnostic() {
        // Arrange
        let error = HostError::Backend("uniqueContextId not found".into());

        // Act
        let terminal = terminal_context_loss(error);

        // Assert
        assert!(matches!(
            terminal,
            HostError::Backend(message) if message.starts_with("Execution context was destroyed")
        ));
    }

    #[test]
    fn inactive_target_closed_uses_context_destroyed_diagnostic() {
        // Arrange
        let error = HostError::TargetClosed;

        // Act
        let terminal = terminal_context_loss(error);

        // Assert
        assert!(matches!(
            terminal,
            HostError::Backend(message) if message.starts_with("Execution context was destroyed")
        ));
    }

    #[tokio::test]
    async fn public_deadline_waits_for_started_attempt_to_acknowledge_cancellation() {
        // Arrange
        let side_effects = Arc::new(AtomicU64::new(0));
        let task_side_effects = Arc::clone(&side_effects);
        let (release, released) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let stop = CancellationToken::new();
            let disconnected = CancellationToken::new();
            run_with_deadline(&stop, &disconnected, 1, move |deadline| async move {
                deadline.cancelled().await;
                task_side_effects.fetch_add(1, Ordering::AcqRel);
                released.await.unwrap();
                task_side_effects.fetch_add(1, Ordering::AcqRel);
                Err::<(), _>(HostError::Timeout(1))
            })
            .await
        });
        while side_effects.load(Ordering::Acquire) == 0 {
            tokio::task::yield_now().await;
        }

        // Act
        assert!(!task.is_finished());
        release.send(()).unwrap();
        let result = task.await.unwrap();
        let observed_at_return = side_effects.load(Ordering::Acquire);
        tokio::task::yield_now().await;

        // Assert
        assert!(matches!(result, Err(HostError::Timeout(1))));
        assert_eq!(observed_at_return, 2);
        assert_eq!(side_effects.load(Ordering::Acquire), observed_at_return);
    }

    #[tokio::test]
    async fn completed_operation_does_not_receive_deadline_cancellation() {
        // Arrange
        let stop = CancellationToken::new();
        let disconnected = CancellationToken::new();
        let cancelled = Arc::new(AtomicU64::new(0));
        let operation_cancelled = Arc::clone(&cancelled);

        // Act
        let result = run_with_deadline(&stop, &disconnected, 1_000, move |deadline| async move {
            if deadline.is_cancelled() {
                operation_cancelled.fetch_add(1, Ordering::AcqRel);
            }
            Ok::<_, HostError>(42)
        })
        .await;

        // Assert
        assert_eq!(result.unwrap(), 42);
        assert_eq!(cancelled.load(Ordering::Acquire), 0);
    }
}
