//! Serialize explicit lifecycle transactions with the native topology driver.

use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use pwright_backend_cdp_page_lifecycle::CdpPageClosePoll;
use pwright_cap_page_creation::BrowserContextNewPage;
use pwright_cap_page_lifecycle::PageClose;
use pwright_composition_chromium::{
    AttachedBrowserContextNewPageService, AttachedChromiumComposition,
};
use pwright_core::{CallId, CallMetadata, Command, CommandResult, CommandSpec, ObjectId, Service};
use pwright_runtime_tokio::TokioCleanupExecutor;
use tokio::sync::oneshot;

use crate::context_lifecycle::ContextLifecycleServices;
use crate::frame_metadata::FrameMetadata;
use crate::{CommandRuntime, ContextDescriptor, HostError, HostSession, PageDescriptor};

pub(crate) enum DriverRequest {
    #[cfg(feature = "extra-headers")]
    ExtraHeaders(crate::extra_headers::Request),
    Fence(oneshot::Sender<()>),
    NewContext {
        call_id: CallId,
        browser: ObjectId,
        metadata: Box<crate::context_metadata::ContextReservation>,
        #[cfg(feature = "extra-headers")]
        headers: Option<crate::extra_headers::CreationHeaders>,
        reply: oneshot::Sender<Result<ContextDescriptor, HostError>>,
    },
    CloseContext {
        call_id: CallId,
        context: ObjectId,
        reply: oneshot::Sender<Result<(), HostError>>,
    },
    NewPage {
        call_id: CallId,
        context: ObjectId,
        reply: oneshot::Sender<Result<PageDescriptor, HostError>>,
    },
    ClosePage {
        call_id: CallId,
        page: ObjectId,
        reply: oneshot::Sender<Result<(), HostError>>,
    },
}

pub(crate) enum DriverReply {
    #[cfg(feature = "extra-headers")]
    ExtraHeaders(
        oneshot::Sender<Result<(), HostError>>,
        Result<(), HostError>,
    ),
    Abandoned,
    Fence(oneshot::Sender<()>),
    NewContext(
        oneshot::Sender<Result<ContextDescriptor, HostError>>,
        Result<ContextDescriptor, HostError>,
    ),
    CloseContext(
        oneshot::Sender<Result<(), HostError>>,
        Result<(), HostError>,
    ),
    NewPage {
        #[cfg(feature = "extra-headers")]
        call_id: CallId,
        reply: oneshot::Sender<Result<PageDescriptor, HostError>>,
        result: Result<PageDescriptor, HostError>,
    },
    ClosePage(
        oneshot::Sender<Result<(), HostError>>,
        Result<(), HostError>,
    ),
}

impl DriverReply {
    /// Send only after the driver has reconciled the transaction's native events.
    pub fn send(self) {
        match self {
            #[cfg(feature = "extra-headers")]
            Self::ExtraHeaders(reply, result) => {
                let _ = reply.send(result);
            }
            Self::Abandoned => {}
            Self::Fence(reply) => {
                let _ = reply.send(());
            }
            Self::NewContext(reply, result) => {
                let _ = reply.send(result);
            }
            Self::CloseContext(reply, result) => {
                let _ = reply.send(result);
            }
            Self::NewPage { reply, result, .. } => {
                let _ = reply.send(result);
            }
            Self::ClosePage(reply, result) => {
                let _ = reply.send(result);
            }
        }
    }
}

struct NativeClosePoll(Duration);
impl CdpPageClosePoll for NativeClosePoll {
    fn wait(&self, _completed_checks: usize) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(tokio::time::sleep(self.0))
    }
}

pub(crate) struct LifecycleServices {
    contexts: ContextLifecycleServices,
    create: AttachedBrowserContextNewPageService<TokioCleanupExecutor>,
    close: crate::close_services::PageCloseService,
    metadata: Arc<FrameMetadata>,
    trees: Arc<crate::frame_trees::FrameTrees>,
}

impl LifecycleServices {
    pub fn new(
        composition: &AttachedChromiumComposition,
        runtime: &CommandRuntime,
        cleanup: TokioCleanupExecutor,
        metadata: Arc<FrameMetadata>,
        trees: Arc<crate::frame_trees::FrameTrees>,
        context_metadata: crate::context_metadata::ContextMetadata,
        stop: tokio_util::sync::CancellationToken,
    ) -> Result<Self, HostError> {
        let contexts = ContextLifecycleServices::new(
            composition,
            runtime,
            cleanup.clone(),
            context_metadata,
            trees.clone(),
        )?;
        let create = composition
            .browser_context_new_page_service(runtime, cleanup.clone())
            .map_err(HostError::backend)?
            .ok_or_else(|| {
                HostError::UnsupportedOperation("Page creation profile is not installed".into())
            })?;
        let close = crate::close_services::PageCloseService::new(
            composition,
            runtime,
            cleanup,
            Arc::new(NativeClosePoll(Duration::from_millis(10))),
            &trees,
            stop,
        )?;
        Ok(Self {
            contexts,
            create,
            close,
            metadata,
            trees,
        })
    }

    pub async fn execute(
        &self,
        _state: &std::sync::Mutex<Option<crate::LiveSession>>,
        request: DriverRequest,
    ) -> DriverReply {
        match request {
            #[cfg(feature = "extra-headers")]
            DriverRequest::ExtraHeaders(request) => DriverReply::ExtraHeaders(
                request.reply,
                Err(HostError::Backend(
                    "header request bypassed bounded driver jobs".into(),
                )),
            ),
            DriverRequest::Fence(reply) => DriverReply::Fence(reply),
            DriverRequest::NewContext {
                call_id,
                browser,
                metadata,
                #[cfg(feature = "extra-headers")]
                headers,
                reply,
            } => {
                if reply.is_closed() {
                    return DriverReply::Abandoned;
                }
                DriverReply::NewContext(
                    reply,
                    self.contexts
                        .create(
                            call_id,
                            browser,
                            *metadata,
                            #[cfg(feature = "extra-headers")]
                            headers,
                        )
                        .await,
                )
            }
            DriverRequest::CloseContext {
                call_id,
                context,
                reply,
            } => {
                if reply.is_closed() {
                    return DriverReply::Abandoned;
                }
                DriverReply::CloseContext(reply, self.contexts.close(call_id, context).await)
            }
            DriverRequest::NewPage {
                call_id,
                context,
                reply,
            } => {
                if reply.is_closed() {
                    return DriverReply::Abandoned;
                }
                DriverReply::NewPage {
                    #[cfg(feature = "extra-headers")]
                    call_id,
                    reply,
                    result: self.create_page(call_id, context).await,
                }
            }
            DriverRequest::ClosePage {
                call_id,
                page,
                reply,
            } => {
                if reply.is_closed() {
                    return DriverReply::Abandoned;
                }
                let result = self
                    .close
                    .call(Command::<PageClose>::new(
                        call_id,
                        page,
                        CallMetadata::default(),
                        (),
                    ))
                    .await;
                DriverReply::ClosePage(reply, ordinary_close(outcome(result)))
            }
        }
    }

    #[cfg(feature = "extra-headers")]
    pub async fn settle_header_initialization(
        &self,
        state: &std::sync::Mutex<Option<crate::LiveSession>>,
        mut reply: DriverReply,
    ) -> Result<(DriverReply, bool), HostError> {
        let Some((call_id, page, primary)) = crate::extra_headers::creation_failure(state, &reply)?
        else {
            return Ok((reply, false));
        };
        // Like upstream newPage's failure catch, close the exact created Page,
        // not its Context or the attachment. Reuse the completed creation ID
        // only for this sequential compensation; ordinary close confirms it.
        ordinary_close(outcome(
            self.close
                .call(Command::<PageClose>::new(
                    call_id,
                    page,
                    CallMetadata::default(),
                    (),
                ))
                .await,
        ))
        .map_err(|cleanup| {
            HostError::Backend(format!(
                "Page initialization failed: {primary}; compensation failed: {cleanup}"
            ))
        })?;
        if let DriverReply::NewPage { result, .. } = &mut reply {
            *result = Err(primary);
        }
        Ok((reply, true))
    }

    async fn create_page(
        &self,
        call_id: CallId,
        context: ObjectId,
    ) -> Result<PageDescriptor, HostError> {
        let collection = self.trees.reserve_creation_collection()?;
        let metadata = self.metadata.reserve_new_page()?;
        let reserved_tree = self.trees.reserve_page(metadata.clone())?;
        let created = outcome(
            self.create
                .call(Command::<BrowserContextNewPage>::new(
                    call_id,
                    context,
                    CallMetadata::default(),
                    (),
                ))
                .await,
        )?;
        let initialized = self
            .metadata
            .activate_new_page(created.main_frame_id, metadata.clone())
            .and_then(|()| {
                self.trees
                    .activate_page(reserved_tree, created.object_id, created.main_frame_id)
            });
        let initialized = initialized.and_then(|bindings| {
            collection.commit()?;
            Ok(bindings)
        });
        let (frame_tree, main_frame_binding, frame_tree_binding) = match initialized {
            Ok(tree) => tree,
            Err(error) => {
                // Initialization cannot publish an untracked native Page. Reuse the
                // now-completed call ID only for its sequential compensation call.
                let cleanup = outcome(
                    self.close
                        .call(Command::<PageClose>::new(
                            call_id,
                            created.object_id,
                            CallMetadata::default(),
                            (),
                        ))
                        .await,
                );
                ordinary_close(cleanup).map_err(|cleanup| {
                HostError::Backend(format!(
                    "Page metadata initialization failed: {error}; compensation failed: {cleanup}"
                ))
            })?;
                return Err(error);
            }
        };
        Ok(PageDescriptor {
            id: created.object_id.get().to_string(),
            context_id: created.parent_id.get().to_string(),
            main_frame_id: created.main_frame_id.get().to_string(),
            main_frame_metadata: metadata,
            frame_tree,
            main_frame_binding: Some(Box::new(main_frame_binding)),
            frame_tree_binding,
        })
    }
}

pub(crate) fn outcome<C: CommandSpec>(
    result: Result<CommandResult<C>, Infallible>,
) -> Result<C::Result, HostError> {
    match result {
        Ok(result) => result
            .into_outcome()
            .map_err(|error| HostError::operation(error, 0)),
        Err(never) => match never {},
    }
}

pub(crate) fn ordinary_close(result: Result<(), HostError>) -> Result<(), HostError> {
    match result {
        Err(HostError::TargetClosed) => Ok(()),
        other => other,
    }
}

impl HostSession {
    pub async fn new_page(&self, context_id: &str) -> Result<PageDescriptor, HostError> {
        let (context, call_id, _operation) = self.admit_operation(context_id)?;
        let (reply, completion) = oneshot::channel();
        self.send_driver_request(DriverRequest::NewPage {
            call_id,
            context,
            reply,
        })?;
        self.await_driver_reply(completion).await
    }

    pub async fn close_page(&self, page_id: &str) -> Result<(), HostError> {
        let result = self.close_page_inner(page_id).await;
        settle_page_close(result, &self.stop, &self.disconnected).await
    }

    async fn close_page_inner(&self, page_id: &str) -> Result<(), HostError> {
        let (page, call_id, _operation) = self.admit_operation(page_id)?;
        let (reply, completion) = oneshot::channel();
        self.send_driver_request(DriverRequest::ClosePage {
            call_id,
            page,
            reply,
        })?;
        self.await_driver_reply(completion).await
    }

    pub(crate) async fn await_driver_reply<T>(
        &self,
        completion: oneshot::Receiver<Result<T, HostError>>,
    ) -> Result<T, HostError> {
        tokio::select! {
            biased;
            () = self.stop.cancelled() => Err(HostError::TargetClosed),
            result = completion => complete_driver_reply(result, &self.stop),
        }
    }

    pub(crate) fn send_driver_request(&self, request: DriverRequest) -> Result<(), HostError> {
        self.topology_barriers
            .try_send(request)
            .map_err(|error| failed_driver_admission(error, &self.stop))
    }
}

fn failed_driver_admission(
    error: tokio::sync::mpsc::error::TrySendError<DriverRequest>,
    stop: &tokio_util::sync::CancellationToken,
) -> HostError {
    match error {
        tokio::sync::mpsc::error::TrySendError::Full(_) => {
            HostError::ResourceExhausted("topology/lifecycle request capacity exceeded (8)".into())
        }
        tokio::sync::mpsc::error::TrySendError::Closed(_) => {
            // The queue may close before the driver's normal stop block, just
            // like an already-admitted reply producer can disappear first.
            stop.cancel();
            HostError::TargetClosed
        }
    }
}

fn complete_driver_reply<T>(
    reply: Result<Result<T, HostError>, oneshot::error::RecvError>,
    stop: &tokio_util::sync::CancellationToken,
) -> Result<T, HostError> {
    match reply {
        Ok(result) => result,
        Err(_) => {
            // A dropped producer is not an ordinary closed-object result. The
            // driver can drop active/queued senders before reaching its stop
            // block. Seal admission now so close settlement cannot mistake that
            // window for a live connection and publish a pre-terminal fence.
            // The caller still releases its operation token before any drain wait.
            stop.cancel();
            Err(HostError::TargetClosed)
        }
    }
}

async fn settle_page_close(
    result: Result<(), HostError>,
    stop: &tokio_util::sync::CancellationToken,
    disconnected: &tokio_util::sync::CancellationToken,
) -> Result<(), HostError> {
    if matches!(&result, Err(HostError::TargetClosed)) && stop.is_cancelled() {
        // The command's operation token has already left close_page_inner.
        // Wait outside that token: teardown waits for admitted operations before
        // staging terminal Page events. A successful ordinary close must not
        // resolve with a pre-terminal publication fence and retire its JS identity
        // before the Page-close notification can acquire that same emitter.
        disconnected.cancelled().await;
    }
    ordinary_close(result)
}

pub(crate) async fn settle_context_close(
    result: Result<(), HostError>,
    stop: &tokio_util::sync::CancellationToken,
    disconnected: &tokio_util::sync::CancellationToken,
) -> Result<(), HostError> {
    if matches!(&result, Err(HostError::TargetClosed)) && stop.is_cancelled() {
        // Unlike Page.close, upstream does not suppress an in-flight Context
        // close interrupted by connection loss. Preserve that error, but obtain
        // the terminal fence only after releasing the Context operation token.
        // Otherwise JS may retire this Context's Page emitter identities early.
        disconnected.cancelled().await;
        result
    } else {
        ordinary_close(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_driver_queue_seals_admission_before_reporting_closed() {
        // Arrange
        let stop = tokio_util::sync::CancellationToken::new();
        let (reply, _completion) = oneshot::channel();
        let error = tokio::sync::mpsc::error::TrySendError::Closed(DriverRequest::Fence(reply));
        // Act
        let result = failed_driver_admission(error, &stop);
        // Assert
        assert!(matches!(result, HostError::TargetClosed));
        assert!(stop.is_cancelled());
    }

    #[test]
    fn full_driver_queue_preserves_backpressure_without_stopping_the_connection() {
        // Arrange
        let stop = tokio_util::sync::CancellationToken::new();
        let (reply, _completion) = oneshot::channel();
        let error = tokio::sync::mpsc::error::TrySendError::Full(DriverRequest::Fence(reply));
        // Act
        let result = failed_driver_admission(error, &stop);
        // Assert
        assert!(matches!(result, HostError::ResourceExhausted(_)));
        assert!(!stop.is_cancelled());
    }

    #[tokio::test]
    async fn dropped_driver_producer_seals_admission_before_returning_closed() {
        // Arrange
        let stop = tokio_util::sync::CancellationToken::new();
        let (sender, completion) = oneshot::channel::<Result<(), HostError>>();
        drop(sender);
        let reply = completion.await;
        // Act
        let result = complete_driver_reply(reply, &stop);
        // Assert
        assert!(matches!(result, Err(HostError::TargetClosed)));
        assert!(stop.is_cancelled());
    }

    #[test]
    fn ordinary_closed_object_reply_does_not_stop_its_live_connection() {
        // Arrange
        let stop = tokio_util::sync::CancellationToken::new();
        let reply = Ok(Err::<(), _>(HostError::TargetClosed));
        // Act
        let result = complete_driver_reply(reply, &stop);
        // Assert
        assert!(matches!(result, Err(HostError::TargetClosed)));
        assert!(!stop.is_cancelled());
    }

    #[tokio::test]
    async fn interrupted_context_close_waits_for_publication_then_preserves_closed_error() {
        // Arrange
        let stop = tokio_util::sync::CancellationToken::new();
        let disconnected = tokio_util::sync::CancellationToken::new();
        stop.cancel();
        let mut future = std::pin::pin!(settle_context_close(
            Err(HostError::TargetClosed),
            &stop,
            &disconnected
        ));
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(future.as_mut().poll(&mut context).is_pending());
        // Act
        disconnected.cancel();
        let result = future.await;
        // Assert
        assert!(matches!(result, Err(HostError::TargetClosed)));
    }

    #[tokio::test]
    async fn closed_context_on_a_live_connection_does_not_wait_or_reject() {
        // Arrange
        let stop = tokio_util::sync::CancellationToken::new();
        let disconnected = tokio_util::sync::CancellationToken::new();
        let mut future = std::pin::pin!(settle_context_close(
            Err(HostError::TargetClosed),
            &stop,
            &disconnected
        ));
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        // Act
        let result = future.as_mut().poll(&mut context);
        // Assert
        assert!(matches!(result, std::task::Poll::Ready(Ok(()))));
    }

    #[tokio::test]
    async fn interrupted_page_close_waits_for_terminal_publication() {
        // Arrange
        let stop = tokio_util::sync::CancellationToken::new();
        let disconnected = tokio_util::sync::CancellationToken::new();
        stop.cancel();
        let mut future = std::pin::pin!(settle_page_close(
            Err(HostError::TargetClosed),
            &stop,
            &disconnected,
        ));
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(future.as_mut().poll(&mut context).is_pending());
        // Act
        disconnected.cancel();
        let result = future.await;
        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn already_closed_page_does_not_wait_for_a_live_browser_to_disconnect() {
        // Arrange
        let stop = tokio_util::sync::CancellationToken::new();
        let disconnected = tokio_util::sync::CancellationToken::new();
        let mut future = std::pin::pin!(settle_page_close(
            Err(HostError::TargetClosed),
            &stop,
            &disconnected,
        ));
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        // Act
        let result = future.as_mut().poll(&mut context);
        // Assert
        assert!(matches!(result, std::task::Poll::Ready(Ok(()))));
    }

    #[tokio::test]
    async fn interrupted_close_preserves_nonterminal_operation_errors() {
        // Arrange
        let stop = tokio_util::sync::CancellationToken::new();
        let disconnected = tokio_util::sync::CancellationToken::new();
        stop.cancel();
        let mut future = std::pin::pin!(settle_page_close(
            Err(HostError::Backend("native receipt failed".into())),
            &stop,
            &disconnected,
        ));
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        // Act
        let result = future.as_mut().poll(&mut context);
        // Assert
        assert!(matches!(
            result,
            std::task::Poll::Ready(Err(HostError::Backend(_)))
        ));
    }

    #[test]
    fn ordinary_close_suppresses_only_target_closed() {
        // Arrange
        let closed = Err(HostError::TargetClosed);
        // Act
        let result = ordinary_close(closed);
        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn ordinary_close_does_not_hide_capacity_failure() {
        // Arrange
        let failure = Err(HostError::ResourceExhausted("full".into()));
        // Act
        let result = ordinary_close(failure);
        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    }
}
