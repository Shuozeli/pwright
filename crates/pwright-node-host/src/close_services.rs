//! Explicit native composition choices; ordinary safe-host service layout stays cold.
use crate::{CommandRuntime, HostError, frame_trees::FrameTrees};
use pwright_backend_cdp_page_lifecycle::CdpPageClosePoll;
use pwright_backend_cdp_page_lifecycle::native_confirmation::{
    CloseInterruptionFuture, INTERRUPTION_FUTURE_BYTES, NativeCloseInterruption,
};
use pwright_cap_context_lifecycle::BrowserContextClose;
use pwright_cap_page_lifecycle::PageClose;
use pwright_composition_chromium::{
    AttachedBrowserContextCloseService, AttachedChromiumComposition, AttachedPageCloseService,
    ConfirmedAttachedPageCloseService, ObservedAttachedContextCloseService,
};
use pwright_core::{Command, CommandResult, Service};
use pwright_runtime_tokio::TokioCleanupExecutor;
use std::{convert::Infallible, sync::Arc};

pub(crate) struct NativeStop(pub(crate) tokio_util::sync::CancellationToken);
impl NativeCloseInterruption for NativeStop {
    fn is_interrupted(&self) -> bool {
        self.0.is_cancelled()
    }
    fn interrupted(
        &self,
    ) -> Result<
        CloseInterruptionFuture,
        pwright_core::OperationError<pwright_cap_page_lifecycle::PageLifecycleErrorDetails>,
    > {
        let future = self.0.clone().cancelled_owned();
        if std::mem::size_of_val(&future) > INTERRUPTION_FUTURE_BYTES {
            return Err(pwright_core::OperationError::new(
                pwright_core::FailureKind::ResourceExhausted,
                "native close interruption exceeds admission",
                None,
            ));
        }
        Ok(Box::pin(future))
    }
}

pub(crate) enum PageCloseService {
    Ordinary(AttachedPageCloseService<TokioCleanupExecutor>),
    Confirmed(ConfirmedAttachedPageCloseService<TokioCleanupExecutor>),
}

pub(crate) fn native_profile() -> Result<
    pwright_backend_cdp_page_lifecycle::native_confirmation::NativePageCloseProfile,
    HostError,
> {
    pwright_backend_cdp_page_lifecycle::native_confirmation::NativePageCloseProfile::new(
        pwright_core::ResourceScope::extension(134, 1),
        1,
        128,
    )
    .and_then(|profile| profile.with_installation_owner_bytes(
        std::mem::size_of::<pwright_composition_chromium::ConfirmedChromiumPageCloseCommandService>()
            + std::mem::size_of::<pwright_backend_cdp_page_lifecycle::native_confirmation::NativePageCloseConfirmation>()
            + 4 * std::mem::size_of::<usize>(),
    ))
    .map_err(HostError::backend)
}
impl PageCloseService {
    pub fn new(
        composition: &AttachedChromiumComposition,
        runtime: &CommandRuntime,
        cleanup: TokioCleanupExecutor,
        poll: Arc<dyn CdpPageClosePoll>,
        trees: &Arc<FrameTrees>,
        stop: tokio_util::sync::CancellationToken,
    ) -> Result<Self, HostError> {
        let selected = match trees.close_observer()? {
            Some(observer) => {
                let confirmation = pwright_backend_cdp_page_lifecycle::native_confirmation::NativePageCloseConfirmation::new(
                    composition.connection().clone(), composition.resources(), native_profile()?, NativeStop(stop),
                ).map_err(HostError::backend)?;
                composition
                    .confirmed_page_close_service(
                        runtime,
                        cleanup,
                        poll,
                        observer,
                        Arc::new(confirmation),
                    )
                    .map(Self::Confirmed)
            }
            None => composition
                .page_lifecycle_services(runtime, cleanup, poll)
                .map(|(close, _)| Self::Ordinary(close)),
        };
        selected.ok_or_else(|| {
            HostError::UnsupportedOperation("Page lifecycle profile is not installed".into())
        })
    }
    pub async fn call(
        &self,
        command: Command<PageClose>,
    ) -> Result<CommandResult<PageClose>, Infallible> {
        match self {
            Self::Ordinary(service) => service.call(command).await,
            Self::Confirmed(service) => service.call(command).await,
        }
    }
}

pub(crate) enum ContextCloseService {
    Ordinary(AttachedBrowserContextCloseService<TokioCleanupExecutor>),
    Observed(ObservedAttachedContextCloseService<TokioCleanupExecutor>),
}
impl ContextCloseService {
    pub fn new(
        composition: &AttachedChromiumComposition,
        runtime: &CommandRuntime,
        cleanup: TokioCleanupExecutor,
        trees: &Arc<FrameTrees>,
    ) -> Result<Self, HostError> {
        let selected = match trees.close_observer()? {
            Some(observer) => composition
                .observed_browser_context_close_service_for_page_runtime(runtime, cleanup, observer)
                .map(Self::Observed),
            None => composition
                .browser_context_close_service_for_page_runtime(runtime, cleanup)
                .map(Self::Ordinary),
        };
        selected.ok_or_else(|| {
            HostError::UnsupportedOperation("Context lifecycle profile is not installed".into())
        })
    }
    pub async fn call(
        &self,
        command: Command<BrowserContextClose>,
    ) -> Result<CommandResult<BrowserContextClose>, Infallible> {
        match self {
            Self::Ordinary(service) => service.call(command).await,
            Self::Observed(service) => service.call(command).await,
        }
    }
}
