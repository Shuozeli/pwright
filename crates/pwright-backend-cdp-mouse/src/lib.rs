//! Optional typed Mouse starters with explicit result and native retirement.
//! No ordinary unit-returning Service adapter is provided: dropping such a
//! result future could discard a click's pending native replies.
use pwright_backend_cdp_input::{
    CdpInputCoordinator, CdpInputPolicy, CdpInputTransaction, CdpMouseDispatcher,
    MouseButtonExecution, MouseClickExecution, MouseClickOptions,
};
use pwright_cap_mouse::*;
use pwright_cdp::{CdpClient, connection::CdpError};
use pwright_core::{
    FailureKind, OperationError, ProductionResourceError, ProductionResourceLedger,
};
use std::{
    future::{Future, poll_fn},
    pin::Pin,
    sync::Arc,
    time::Duration,
};

mod errors;
mod manifest;
mod profile;
use errors::{native_error, resource_error};
pub use manifest::*;
use profile::{Budget, Lease};
pub use profile::{CdpMouseProductionProfile, CdpMouseProfileError, MOUSE_ERROR_MAX_BYTES};

pub type MouseResult = Result<(), OperationError<MouseErrorDetails>>;
pub type MouseNativeFuture = Pin<Box<dyn Future<Output = Result<(), CdpError>> + Send>>;
type DelayFactory = Box<dyn FnMut(Duration) -> MouseNativeFuture + Send>;
type Sequence = MouseClickExecution<DelayFactory, MouseNativeFuture>;

/// The provider owns/accountably bounds its returned future and supplies time.
/// This leaf owns no timer, task, runtime, CDP connection or browser process.
pub trait CdpMouseDelay: Send + Sync {
    fn wait(self: Arc<Self>, duration: Duration) -> MouseNativeFuture;
}
#[derive(Clone)]
pub struct CdpMouseServiceFactory {
    budget: Budget,
    delay: Arc<dyn CdpMouseDelay>,
}
impl CdpMouseServiceFactory {
    pub fn new_with_production_resources(
        profile: CdpMouseProductionProfile,
        resources: &ProductionResourceLedger,
        delay: Arc<dyn CdpMouseDelay>,
    ) -> Result<Self, ProductionResourceError> {
        Ok(Self {
            budget: Budget::new(profile, resources)?,
            delay,
        })
    }
    pub fn active_operations(&self) -> usize {
        self.budget.active()
    }
    pub fn profile(&self) -> CdpMouseProductionProfile {
        self.budget.profile
    }
    pub fn bind(
        &self,
        client: Arc<dyn CdpClient>,
        input: CdpInputCoordinator,
    ) -> Result<CdpMouseServices, OperationError<MouseErrorDetails>> {
        self.bind_selected(client, input, None)
    }
    /// Select a single Page-scoped dispatcher for move/down/up/click/dblclick.
    /// Wheel remains direct. The provider must separately admit its state and
    /// operation trees and keep persistent state owned through Page shutdown.
    pub fn bind_with_dispatcher(
        &self,
        client: Arc<dyn CdpClient>,
        input: CdpInputCoordinator,
        dispatcher: Arc<dyn CdpMouseDispatcher>,
    ) -> Result<CdpMouseServices, OperationError<MouseErrorDetails>> {
        self.bind_selected(client, input, Some(dispatcher))
    }
    fn bind_selected(
        &self,
        client: Arc<dyn CdpClient>,
        input: CdpInputCoordinator,
        dispatcher: Option<Arc<dyn CdpMouseDispatcher>>,
    ) -> Result<CdpMouseServices, OperationError<MouseErrorDetails>> {
        let required = CdpMouseServices::required_operation_bytes(client.clone(), &input)?;
        if self.profile().operation_bytes() < required {
            return Err(resource_error(
                "Mouse profile does not cover concrete operation storage",
            ));
        }
        let page = Arc::new(PageInput {
            client,
            input,
            budget: self.budget.clone(),
            delay: self.delay.clone(),
            dispatcher,
        });
        Ok(CdpMouseServices {
            mouse_move: CdpMouseMoveService(page.clone()),
            mouse_down: CdpMouseDownService(page.clone()),
            mouse_up: CdpMouseUpService(page.clone()),
            mouse_click: CdpMouseClickService(page.clone()),
            mouse_dblclick: CdpMouseDblclickService(page.clone()),
            mouse_wheel: CdpMouseWheelService(page),
        })
    }
}
struct PageInput {
    client: Arc<dyn CdpClient>,
    input: CdpInputCoordinator,
    budget: Budget,
    delay: Arc<dyn CdpMouseDelay>,
    dispatcher: Option<Arc<dyn CdpMouseDispatcher>>,
}
impl PageInput {
    fn start(
        &self,
        operation: NativeOperation,
    ) -> Result<CdpMouseOperation, OperationError<MouseErrorDetails>> {
        let lease = self.budget.reserve()?;
        let mut transaction = self
            .input
            .begin_raw_mouse_transaction(self.client.clone())
            .map_err(native_error)?;
        let execution = match operation {
            NativeOperation::Sequence(options) => {
                let delay = self.delay.clone();
                let factory: DelayFactory = Box::new(move |duration| delay.clone().wait(duration));
                let sequence = match &self.dispatcher {
                    Some(dispatcher) => transaction.raw_mouse_click_with_dispatcher(
                        options,
                        factory,
                        dispatcher.clone(),
                    ),
                    None => transaction.raw_mouse_click(options, factory),
                }
                .map_err(native_error)?;
                Execution::Sequence(Box::pin(sequence))
            }
            NativeOperation::Single(SingleAction::Down(button, count))
                if self.dispatcher.is_some() =>
            {
                let button = transaction
                    .raw_mouse_down_with_dispatcher(
                        native_button(button),
                        count,
                        self.dispatcher
                            .as_ref()
                            .expect("selected dispatcher")
                            .clone(),
                    )
                    .map_err(native_error)?;
                Execution::Button(Box::pin(button))
            }
            NativeOperation::Single(SingleAction::Up(button, count))
                if self.dispatcher.is_some() =>
            {
                let button = transaction
                    .raw_mouse_up_with_dispatcher(
                        native_button(button),
                        count,
                        self.dispatcher
                            .as_ref()
                            .expect("selected dispatcher")
                            .clone(),
                    )
                    .map_err(native_error)?;
                Execution::Button(Box::pin(button))
            }
            NativeOperation::Single(action) => Execution::Single {
                future: Some(Box::pin(single(transaction, action))),
                started: false,
            },
        };
        Ok(CdpMouseOperation {
            execution,
            observed: false,
            _lease: lease,
        })
    }
}
enum NativeOperation {
    Sequence(MouseClickOptions),
    Single(SingleAction),
}
#[derive(Clone, Copy)]
enum SingleAction {
    Down(MouseButton, i32),
    Up(MouseButton, i32),
    Wheel(f64, f64),
}
async fn single(
    mut transaction: CdpInputTransaction,
    action: SingleAction,
) -> Result<(), CdpError> {
    let result = match action {
        SingleAction::Down(button, count) => {
            transaction
                .raw_mouse_down(native_button(button), count)
                .await
        }
        SingleAction::Up(button, count) => {
            transaction.raw_mouse_up(native_button(button), count).await
        }
        SingleAction::Wheel(x, y) => transaction.raw_mouse_wheel(x, y).await,
    };
    // Raw Interleaved transactions have no requested modifier restoration or
    // implicit release. This finish does not synthesize additional input.
    transaction.finish().await?;
    result
}
enum Execution {
    Sequence(Pin<Box<Sequence>>),
    Button(Pin<Box<MouseButtonExecution>>),
    Single {
        future: Option<MouseNativeFuture>,
        started: bool,
    },
}
/// Caller-owned native operation. Always retain it through retirement, including
/// after an error or cancellation of result polling. Dropping is NOT retirement.
/// The route/host must retain this owner outside any cancellable response future.
#[must_use = "retain the Mouse operation, await its result by borrow, then retire"]
pub struct CdpMouseOperation {
    // Allocation owners precede the admission lease in drop order.
    execution: Execution,
    observed: bool,
    _lease: Lease,
}
impl CdpMouseOperation {
    pub fn pending_native_calls(&self) -> usize {
        match &self.execution {
            Execution::Sequence(sequence) => sequence.pending_native_calls(),
            Execution::Button(button) => button.pending_native_calls(),
            Execution::Single { future, started } => usize::from(*started && future.is_some()),
        }
    }
    pub async fn result(&mut self) -> MouseResult {
        if self.observed {
            return Err(OperationError::new(
                FailureKind::ProtocolViolation,
                "Mouse result was already observed or cancelled",
                None,
            ));
        }
        let result = poll_fn(|cx| match &mut self.execution {
            Execution::Sequence(sequence) => sequence.as_mut().poll(cx),
            Execution::Button(button) => button.as_mut().poll(cx),
            Execution::Single { future, started } => {
                *started = true;
                let result = future
                    .as_mut()
                    .expect("unobserved single operation retains future")
                    .as_mut()
                    .poll(cx);
                if result.is_ready() {
                    *future = None;
                }
                result
            }
        })
        .await;
        self.observed = true;
        result.map_err(native_error)
    }
    /// Stops planning, drains already-started native requests, and never releases
    /// physical buttons implicitly. May itself be borrowed and resumed if polling
    /// is interrupted. Admission remains attached until this owner is dropped.
    pub async fn retire(&mut self) -> MouseResult {
        self.observed = true;
        let result = match &mut self.execution {
            Execution::Sequence(sequence) => sequence.retire().await,
            Execution::Button(button) => button.retire().await,
            Execution::Single { future, started } => {
                let result = if *started {
                    match future.as_mut() {
                        Some(future) => future.await,
                        None => Ok(()),
                    }
                } else {
                    Ok(())
                };
                *future = None;
                result
            }
        };
        result.map_err(|error| native_error(error).reclassify(FailureKind::Cleanup))
    }
}

#[derive(Clone)]
pub struct CdpMouseServices {
    pub mouse_move: CdpMouseMoveService,
    pub mouse_down: CdpMouseDownService,
    pub mouse_up: CdpMouseUpService,
    pub mouse_click: CdpMouseClickService,
    pub mouse_dblclick: CdpMouseDblclickService,
    pub mouse_wheel: CdpMouseWheelService,
}
impl CdpMouseServices {
    /// Payload allowance for the concrete wrapper/sequence/single future, boxed
    /// delay closure and two bounded diagnostic strings. Excludes allocator
    /// headers, Page binding/state, and allocations inside injected native/delay
    /// providers (including their returned errors). Providers need separate
    /// admission; this is not a process-memory or full native-retirement proof.
    pub fn required_operation_bytes(
        client: Arc<dyn CdpClient>,
        input: &CdpInputCoordinator,
    ) -> Result<usize, OperationError<MouseErrorDetails>> {
        if input.policy() != CdpInputPolicy::Interleaved {
            return Err(OperationError::new(
                FailureKind::UnsupportedCapability,
                "Mouse requires shared Interleaved Page input",
                None,
            ));
        }
        let transaction = input
            .begin_raw_mouse_transaction(client.clone())
            .map_err(native_error)?;
        let future = single(transaction, SingleAction::Down(MouseButton::Left, 1));
        let single_bytes = std::mem::size_of_val(&future);
        let sequence_bytes = Sequence::allocation_payload_bytes(client)
            + std::mem::size_of::<Arc<dyn CdpMouseDelay>>();
        Ok(std::mem::size_of::<CdpMouseOperation>()
            + single_bytes
                .max(sequence_bytes)
                .max(MouseButtonExecution::allocation_payload_bytes())
            + 2 * MOUSE_ERROR_MAX_BYTES)
    }
    /// Shared Page binding payload and Arc counters; route accounts for this.
    pub const fn shared_binding_bytes() -> usize {
        std::mem::size_of::<PageInput>() + 2 * std::mem::size_of::<usize>()
    }
}
fn native_button(button: MouseButton) -> pwright_cdp::MouseButton {
    match button {
        MouseButton::Left => pwright_cdp::MouseButton::Left,
        MouseButton::Right => pwright_cdp::MouseButton::Right,
        MouseButton::Middle => pwright_cdp::MouseButton::Middle,
    }
}
fn click_options(
    x: MouseNumber,
    y: MouseNumber,
    button: MouseButton,
    count: MouseClickCount,
    delay: Option<Duration>,
) -> MouseClickOptions {
    MouseClickOptions::new(x.get(), y.get())
        .expect("typed coordinates are finite")
        .with_button(native_button(button))
        .with_count(count)
        .with_delay(delay.unwrap_or(Duration::ZERO))
}
macro_rules! mouse_starter {
    ($service:ident,$params:ty,$convert:expr) => {
        #[derive(Clone)]
        pub struct $service(Arc<PageInput>);
        impl $service {
            pub fn shares_input_state_with(&self, input: &CdpInputCoordinator) -> bool {
                self.0.input.shares_state_with(input)
            }
            pub fn start(
                &self,
                params: $params,
            ) -> Result<CdpMouseOperation, OperationError<MouseErrorDetails>> {
                self.0.start(($convert)(params))
            }
        }
        impl pwright_browser_backend::NativeBackendAccess<ChromiumCdpMouseBackend> for $service {
            type Native = dyn CdpClient;
            fn native_backend(&self) -> &Self::Native {
                self.0.client.as_ref()
            }
        }
    };
}
mouse_starter!(CdpMouseMoveService, MoveParams, |p: MoveParams| {
    NativeOperation::Sequence(
        MouseClickOptions::new(p.x.get(), p.y.get())
            .expect("finite coordinates")
            .with_count(MouseClickCount::try_from(0.0).expect("zero is a valid count"))
            .with_steps(p.steps)
            .with_for_click(false),
    )
});
mouse_starter!(CdpMouseDownService, DownParams, |p: DownParams| {
    NativeOperation::Single(SingleAction::Down(p.button, p.click_count))
});
mouse_starter!(CdpMouseUpService, UpParams, |p: UpParams| {
    NativeOperation::Single(SingleAction::Up(p.button, p.click_count))
});
mouse_starter!(CdpMouseClickService, ClickParams, |p: ClickParams| {
    NativeOperation::Sequence(click_options(p.x, p.y, p.button, p.click_count, p.delay))
});
mouse_starter!(
    CdpMouseDblclickService,
    DblclickParams,
    |p: DblclickParams| NativeOperation::Sequence(click_options(
        p.x,
        p.y,
        p.button,
        MouseClickCount::TWO,
        p.delay
    ))
);
mouse_starter!(CdpMouseWheelService, WheelParams, |p: WheelParams| {
    NativeOperation::Single(SingleAction::Wheel(p.delta_x.get(), p.delta_y.get()))
});
#[cfg(test)]
mod tests;
