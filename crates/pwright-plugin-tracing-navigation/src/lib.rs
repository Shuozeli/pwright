//! Optional API-call tracing around the Playwright-shaped navigation command.
//!
//! This leaf plugin contributes only correlated `navigation.goto` before and
//! terminal metadata. It owns no browser, transport, sink, runtime, task, DOM
//! snapshot, screenshot, HAR body, source file, or archive writer.

use std::convert::Infallible;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;

use pwright_cap_navigation::{Goto, NavigationErrorDetails};
use pwright_core::{Command, CommandResult, FailureKind, Layer, ObjectId, OperationError, Service};
use pwright_trace_model::TraceEntryKind;
use pwright_tracing_service::{TraceClock, TraceSink, TracingService};

pub trait NavigationTraceRoute: Send + Sync {
    fn browser_context_for_page(&self, page_id: ObjectId) -> Option<ObjectId>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FixedNavigationTraceRoute {
    page_id: ObjectId,
    context_id: ObjectId,
}

impl FixedNavigationTraceRoute {
    pub const fn new(page_id: ObjectId, context_id: ObjectId) -> Self {
        Self {
            page_id,
            context_id,
        }
    }
}

impl NavigationTraceRoute for FixedNavigationTraceRoute {
    fn browser_context_for_page(&self, page_id: ObjectId) -> Option<ObjectId> {
        (page_id == self.page_id).then_some(self.context_id)
    }
}

pub struct NavigationTracingLayer<Clock, Sink, Route> {
    tracing: Arc<TracingService<Clock, Sink>>,
    routes: Arc<Route>,
}

impl<Clock, Sink, Route> NavigationTracingLayer<Clock, Sink, Route> {
    pub fn new(tracing: Arc<TracingService<Clock, Sink>>, routes: Arc<Route>) -> Self {
        Self { tracing, routes }
    }
}

impl<Clock, Sink, Route, Inner> Layer<Inner> for NavigationTracingLayer<Clock, Sink, Route> {
    type Service = NavigationTracingService<Inner, Clock, Sink, Route>;

    fn layer(&self, inner: Inner) -> Self::Service {
        NavigationTracingService {
            inner,
            tracing: Arc::clone(&self.tracing),
            routes: Arc::clone(&self.routes),
            marker: PhantomData,
        }
    }
}

pub struct NavigationTracingService<Inner, Clock, Sink, Route> {
    inner: Inner,
    tracing: Arc<TracingService<Clock, Sink>>,
    routes: Arc<Route>,
    marker: PhantomData<fn() -> Goto>,
}

impl<Inner, Clock, Sink, Route> NavigationTracingService<Inner, Clock, Sink, Route> {
    pub const fn inner(&self) -> &Inner {
        &self.inner
    }
}

impl<Inner, Clock, Sink, Route> Service<Command<Goto>>
    for NavigationTracingService<Inner, Clock, Sink, Route>
where
    Inner: Service<Command<Goto>, Response = CommandResult<Goto>, Error = Infallible>,
    Clock: TraceClock + 'static,
    Sink: TraceSink + 'static,
    Route: NavigationTraceRoute + 'static,
{
    type Response = CommandResult<Goto>;
    type Error = Infallible;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, command: Command<Goto>) -> Self::Future<'_> {
        let call_id = command.call_id();
        let page_id = command.object_id();
        let context_id = self.routes.browser_context_for_page(page_id);
        Box::pin(async move {
            let Some(context_id) = context_id else {
                return Ok(CommandResult::new(
                    call_id,
                    Err(OperationError::new(
                        FailureKind::TargetClosed,
                        format!("Page {page_id} has no BrowserContext trace route"),
                        Some(NavigationErrorDetails::Backend(
                            "navigation tracing could not resolve the target Context".to_owned(),
                        )),
                    )),
                ));
            };
            let tracing_active = match self.tracing.record_api_call_if_recording(
                context_id,
                TraceEntryKind::ApiCallBefore,
                call_id,
                "navigation.goto",
            ) {
                Ok(sequence) => sequence.is_some(),
                Err(error) => return Ok(trace_failure(call_id, error)),
            };
            let result = self.inner.call(command).await?;
            if tracing_active {
                let terminal = if result.outcome().is_ok() {
                    TraceEntryKind::ApiCallAfter
                } else {
                    TraceEntryKind::ApiCallError
                };
                if let Err(error) = self.tracing.record_api_call_if_recording(
                    context_id,
                    terminal,
                    call_id,
                    "navigation.goto",
                ) {
                    return Ok(trace_failure(call_id, error));
                }
            }
            Ok(result)
        })
    }
}

fn trace_failure(
    call_id: pwright_core::CallId,
    error: OperationError<pwright_cap_tracing::TracingErrorDetails>,
) -> CommandResult<Goto> {
    CommandResult::new(
        call_id,
        Err(OperationError::new(
            error.kind(),
            "navigation trace contribution failed",
            Some(NavigationErrorDetails::Backend(error.to_string())),
        )),
    )
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::future::{Ready, ready};
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU64, Ordering};

    use pwright_cap_navigation::{GotoParams, GotoResult};
    use pwright_cap_tracing::{
        TraceStart, TraceStartChunk, TraceStartChunkOptions, TraceStartOptions, TraceStopChunk,
        TraceStopChunkMode, TraceStopChunkOptions,
    };
    use pwright_core::{
        CallId, CallMetadata, Command, ProductionResourceLedger, ResourceClass, ResourceLimits,
        ResourceScope, Service, Stack,
    };
    use pwright_trace_model::{TraceChunk, TraceLimits, TraceRecorderProductionProfile};

    use super::*;

    struct TestClock(AtomicU64);

    impl TraceClock for TestClock {
        fn monotonic_micros(&self) -> u64 {
            self.0.fetch_add(1, Ordering::Relaxed)
        }
    }

    type RecordedEntries = Arc<Mutex<Vec<(TraceEntryKind, Vec<u8>)>>>;

    #[derive(Clone, Default)]
    struct RecordingSink {
        entries: RecordedEntries,
    }

    impl TraceSink for RecordingSink {
        type Error = Infallible;

        fn write(&self, _: ObjectId, chunk: TraceChunk) -> Result<(), Self::Error> {
            self.entries.lock().unwrap().extend(
                chunk
                    .entries()
                    .iter()
                    .map(|entry| (entry.kind(), entry.payload().to_vec())),
            );
            Ok(())
        }

        fn abort_context(&self, _: ObjectId) -> Result<(), Self::Error> {
            Ok(())
        }

        fn abort_all(&self) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    struct SuccessfulGoto;

    impl Service<Command<Goto>> for SuccessfulGoto {
        type Response = CommandResult<Goto>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<Goto>) -> Self::Future<'_> {
            let (call_id, _, _, params) = command.into_parts();
            ready(Ok(CommandResult::new(
                call_id,
                Ok(GotoResult {
                    frame_id: "frame".to_owned(),
                    loader_id: Some("loader".to_owned()),
                    url: params.url,
                    same_document: false,
                    response: None,
                }),
            )))
        }
    }

    fn setup() -> (
        Arc<TracingService<TestClock, RecordingSink>>,
        RecordingSink,
        ProductionResourceLedger,
        ObjectId,
        ObjectId,
    ) {
        let context_id = ObjectId::new(2);
        let page_id = ObjectId::new(3);
        let profile = TraceRecorderProductionProfile::new(
            TraceLimits::new(32, 4, 64, 256, 1),
            ResourceScope::Object(context_id),
        )
        .unwrap();
        let requirements = profile.requirements();
        let trace_bytes = requirements.class(ResourceClass::TraceBytes);
        let resources = ProductionResourceLedger::new(
            ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
                ResourceClass::TraceBytes,
                trace_bytes.global,
                trace_bytes.per_scope,
            ),
            requirements,
        )
        .unwrap();
        let recorder = profile.create(&resources).unwrap();
        let sink = RecordingSink::default();
        let tracing =
            Arc::new(TracingService::new(TestClock(AtomicU64::new(10)), sink.clone(), 1).unwrap());
        tracing.register_context(context_id, recorder).unwrap();
        (tracing, sink, resources, context_id, page_id)
    }

    fn command<Spec: pwright_core::CommandSpec>(
        call_id: u64,
        object_id: ObjectId,
        params: Spec::Params,
    ) -> Command<Spec> {
        Command::new(
            CallId::new(call_id),
            object_id,
            CallMetadata::default(),
            params,
        )
    }

    #[tokio::test]
    async fn installed_plugin_is_inert_until_the_user_starts_a_chunk() {
        // Arrange
        let (tracing, sink, resources, context_id, page_id) = setup();
        let service = Stack::new(SuccessfulGoto).layer(NavigationTracingLayer::new(
            tracing,
            Arc::new(FixedNavigationTraceRoute::new(page_id, context_id)),
        ));

        // Act
        let result = service
            .call(command::<Goto>(
                40,
                page_id,
                GotoParams::new("https://example.test/inert"),
            ))
            .await
            .unwrap();

        // Assert
        assert!(result.outcome().is_ok());
        assert!(sink.entries.lock().unwrap().is_empty());
        assert_eq!(
            resources
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
    }

    #[tokio::test]
    async fn active_chunk_gets_one_correlated_before_and_after_pair() {
        // Arrange
        let (tracing, sink, resources, context_id, page_id) = setup();
        Service::<Command<TraceStart>>::call(
            tracing.as_ref(),
            command(1, context_id, TraceStartOptions::new(None)),
        )
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
        Service::<Command<TraceStartChunk>>::call(
            tracing.as_ref(),
            command(2, context_id, TraceStartChunkOptions::new(None, None)),
        )
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
        let service = Stack::new(SuccessfulGoto).layer(NavigationTracingLayer::new(
            Arc::clone(&tracing),
            Arc::new(FixedNavigationTraceRoute::new(page_id, context_id)),
        ));

        // Act
        let result = service
            .call(command::<Goto>(
                42,
                page_id,
                GotoParams::new("https://example.test/traced"),
            ))
            .await
            .unwrap();
        Service::<Command<TraceStopChunk>>::call(
            tracing.as_ref(),
            command(
                3,
                context_id,
                TraceStopChunkOptions::new(TraceStopChunkMode::Sink),
            ),
        )
        .await
        .unwrap()
        .into_outcome()
        .unwrap();

        // Assert
        assert!(result.outcome().is_ok());
        let entries = sink.entries.lock().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0, TraceEntryKind::ApiCallBefore);
        assert_eq!(entries[1].0, TraceEntryKind::ApiCallAfter);
        assert_eq!(entries[0].1, entries[1].1);
        assert_eq!(
            u64::from_be_bytes(entries[0].1[1..9].try_into().unwrap()),
            42
        );
        drop(entries);
        assert_eq!(
            resources
                .metrics()
                .unwrap()
                .class(ResourceClass::TraceBytes)
                .current,
            0
        );
    }
}
