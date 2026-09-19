//! Single-call bridge to an injected controlled runtime; no executor is owned.
use crate::{ChromiumMousePage, MouseCommandFuture, MouseResult, queue};
use pwright_cap_mouse::*;
use pwright_core::{
    CallMetadata, CancellationReason, Command, CommandCancellation, CommandResult, CommandSpec,
    ObjectId, OperationContext, OperationError, Service,
};
use pwright_runtime::TargetedService;
use std::{
    convert::Infallible,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::{Arc, Mutex},
};

enum State {
    Fresh,
    Cancelled(CancellationReason),
    Started(Option<queue::RetirementTicket>),
}
/// Use with the same command passed to `for_command` and the existing runtime's
/// `dispatch_with`. Backend admission happens only when that runtime starts it.
/// The embedding separately budgets this caller-owned invocation/Arc and boxed
/// futures. It must keep the Page's unique retirement driver alive and consume
/// every receipt via `into_outcome`, including success and early-error receipts.
pub struct ChromiumMouseInvocation<C> {
    page: Arc<ChromiumMousePage>,
    context: OperationContext,
    state: Mutex<State>,
    marker: PhantomData<fn() -> C>,
}
impl<C: CommandSpec<Result = (), ErrorDetails = MouseErrorDetails>> ChromiumMouseInvocation<C> {
    pub fn for_command(
        page: Arc<ChromiumMousePage>,
        command: &Command<C>,
    ) -> Result<Self, OperationError<MouseErrorDetails>> {
        let context = OperationContext::new(
            command.call_id(),
            command.object_id(),
            C::METHOD,
            command.metadata().cancellation,
        );
        page.shared.validate_identity(context.object_id())?;
        Ok(Self {
            page,
            context,
            state: Mutex::new(State::Fresh),
            marker: PhantomData,
        })
    }
    pub const fn context(&self) -> OperationContext {
        self.context
    }
    /// Inline invocation + optional Arc header; excludes caller dispatch future,
    /// executor wakers and allocator headers. Record/ticket state is route-owned.
    pub const fn allocation_bytes() -> usize {
        std::mem::size_of::<Self>() + 2 * std::mem::size_of::<usize>()
    }
}
impl<C> ChromiumMouseInvocation<C>
where
    C: CommandSpec<Result = (), ErrorDetails = MouseErrorDetails>,
    ChromiumMousePage: for<'a> Service<
            Command<C>,
            Response = CommandResult<C>,
            Error = Infallible,
            Future<'a> = MouseCommandFuture<C>,
        >,
{
    fn start(
        &self,
        page_id: ObjectId,
        params: C::Params,
    ) -> Pin<Box<dyn Future<Output = MouseResult> + Send + '_>> {
        if page_id != self.context.object_id() {
            return Box::pin(async { Err(queue::protocol_error("Mouse invocation Page differs")) });
        }
        let mut state = self.state.lock().expect("Mouse invocation poisoned");
        if let State::Cancelled(reason) = *state {
            return Box::pin(async move { Err(queue::cancelled(reason)) });
        }
        if !matches!(*state, State::Fresh) {
            return Box::pin(async {
                Err(queue::protocol_error("Mouse invocation already started"))
            });
        }
        let response = self.page.call(Command::<C>::new(
            self.context.call_id(),
            page_id,
            CallMetadata::new(None, self.context.cancellation()),
            params,
        ));
        *state = State::Started(response.response.retirement_ticket());
        Box::pin(async move {
            match response.await {
                Ok(result) => result.into_outcome(),
                Err(never) => match never {},
            }
        })
    }
}
macro_rules! targeted {
    ($command:ty, $params:ty) => {
        impl TargetedService<$params> for ChromiumMouseInvocation<$command> {
            type Response = ();
            type Error = OperationError<MouseErrorDetails>;
            type Future<'a> = Pin<Box<dyn Future<Output = MouseResult> + Send + 'a>>;
            fn call_for(&self, page_id: ObjectId, params: $params) -> Self::Future<'_> {
                self.start(page_id, params)
            }
        }
    };
}
targeted!(MouseMove, MoveParams);
targeted!(MouseDown, DownParams);
targeted!(MouseUp, UpParams);
targeted!(MouseClick, ClickParams);
targeted!(MouseDblclick, DblclickParams);
targeted!(MouseWheel, WheelParams);
impl<C: CommandSpec<Result = (), ErrorDetails = MouseErrorDetails>> CommandCancellation<C>
    for ChromiumMouseInvocation<C>
{
    type Future<'a>
        = Pin<Box<dyn Future<Output = MouseResult> + Send + 'a>>
    where
        Self: 'a;
    fn cancel(&self, context: OperationContext, reason: CancellationReason) -> Self::Future<'_> {
        if context != self.context {
            return Box::pin(async {
                Err(queue::protocol_error(
                    "Mouse invocation cancellation identity differs",
                ))
            });
        }
        let ticket = {
            let mut state = self.state.lock().expect("Mouse invocation poisoned");
            match &*state {
                State::Fresh => {
                    *state = State::Cancelled(reason);
                    None
                }
                State::Cancelled(_) | State::Started(None) => None,
                State::Started(Some(ticket)) => Some(ticket.clone()),
            }
        };
        Box::pin(async move {
            match ticket {
                // No route admission occurred, so there is no native work to retire.
                None => Ok(()),
                Some(ticket) => ticket.cancel(reason).await,
            }
        })
    }
}
