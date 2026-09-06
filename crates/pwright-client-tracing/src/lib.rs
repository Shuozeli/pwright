//! Opt-in BrowserContext tracing methods for `pwright-client` handles.
//!
//! Importing this crate adds a Playwright-shaped `context.tracing()` facade.
//! It does not install the capability, recorder, sink, or a background task.

use std::convert::Infallible;

use pwright_cap_tracing::{
    TraceGroup, TraceGroupEnd, TraceGroupEndResult, TraceGroupOptions, TraceGroupResult,
    TraceStart, TraceStartChunk, TraceStartChunkOptions, TraceStartChunkResult, TraceStartOptions,
    TraceStartResult, TraceStop, TraceStopChunk, TraceStopChunkOptions, TraceStopChunkResult,
    TraceStopResult, TracingErrorDetails,
};
use pwright_client::{BrowserContext, CallIdSource, CommandFuture};
use pwright_core::{CallMetadata, Command, CommandResult, Service};

pub trait BrowserContextTracingExt<Channel, Ids> {
    fn tracing(&self) -> Tracing<'_, Channel, Ids>;
}

impl<Channel, Ids> BrowserContextTracingExt<Channel, Ids> for BrowserContext<Channel, Ids> {
    fn tracing(&self) -> Tracing<'_, Channel, Ids> {
        Tracing { context: self }
    }
}

#[derive(Clone, Copy)]
pub struct Tracing<'a, Channel, Ids> {
    context: &'a BrowserContext<Channel, Ids>,
}

impl<Channel, Ids> Tracing<'_, Channel, Ids>
where
    Ids: CallIdSource,
{
    pub fn start(
        &self,
        options: TraceStartOptions,
    ) -> CommandFuture<TraceStartResult, TracingErrorDetails>
    where
        Channel: Service<Command<TraceStart>, Response = CommandResult<TraceStart>, Error = Infallible>
            + 'static,
    {
        self.context
            .as_remote()
            .dispatch::<TraceStart>(options, CallMetadata::default())
    }

    pub fn start_chunk(
        &self,
        options: TraceStartChunkOptions,
    ) -> CommandFuture<TraceStartChunkResult, TracingErrorDetails>
    where
        Channel: Service<
                Command<TraceStartChunk>,
                Response = CommandResult<TraceStartChunk>,
                Error = Infallible,
            > + 'static,
    {
        self.context
            .as_remote()
            .dispatch::<TraceStartChunk>(options, CallMetadata::default())
    }

    pub fn group(
        &self,
        options: TraceGroupOptions,
    ) -> CommandFuture<TraceGroupResult, TracingErrorDetails>
    where
        Channel: Service<Command<TraceGroup>, Response = CommandResult<TraceGroup>, Error = Infallible>
            + 'static,
    {
        self.context
            .as_remote()
            .dispatch::<TraceGroup>(options, CallMetadata::default())
    }

    pub fn group_end(&self) -> CommandFuture<TraceGroupEndResult, TracingErrorDetails>
    where
        Channel: Service<
                Command<TraceGroupEnd>,
                Response = CommandResult<TraceGroupEnd>,
                Error = Infallible,
            > + 'static,
    {
        self.context
            .as_remote()
            .dispatch::<TraceGroupEnd>((), CallMetadata::default())
    }

    pub fn stop_chunk(
        &self,
        options: TraceStopChunkOptions,
    ) -> CommandFuture<TraceStopChunkResult, TracingErrorDetails>
    where
        Channel: Service<
                Command<TraceStopChunk>,
                Response = CommandResult<TraceStopChunk>,
                Error = Infallible,
            > + 'static,
    {
        self.context
            .as_remote()
            .dispatch::<TraceStopChunk>(options, CallMetadata::default())
    }

    pub fn stop(&self) -> CommandFuture<TraceStopResult, TracingErrorDetails>
    where
        Channel: Service<Command<TraceStop>, Response = CommandResult<TraceStop>, Error = Infallible>
            + 'static,
    {
        self.context
            .as_remote()
            .dispatch::<TraceStop>((), CallMetadata::default())
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::future::{Ready, ready};
    use std::sync::{Arc, Mutex};

    use pwright_cap_tracing::{
        TraceGroup, TraceGroupEnd, TraceGroupOptions, TraceStart, TraceStartChunk,
        TraceStartChunkOptions, TraceStartChunkResult, TraceStartOptions, TraceStop,
        TraceStopChunk, TraceStopChunkMode, TraceStopChunkOptions, TraceStopChunkResult,
    };
    use pwright_client::{
        AtomicCallIdSource, BrowserContextKind, BrowserKind, Client, RemoteInitializer, RemoteKind,
    };
    use pwright_core::{CallId, Command, CommandResult, ObjectId, Service};

    use super::BrowserContextTracingExt;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum RecordedMethod {
        Start,
        StartChunk,
        Group,
        GroupEnd,
        StopChunk,
        Stop,
    }

    struct RecordingChannel {
        calls: Arc<Mutex<Vec<(RecordedMethod, CallId, ObjectId)>>>,
    }

    macro_rules! impl_unit_service {
        ($spec:ty, $method:expr) => {
            impl Service<Command<$spec>> for RecordingChannel {
                type Response = CommandResult<$spec>;
                type Error = Infallible;
                type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

                fn call(&self, command: Command<$spec>) -> Self::Future<'_> {
                    self.calls.lock().unwrap().push((
                        $method,
                        command.call_id(),
                        command.object_id(),
                    ));
                    ready(Ok(CommandResult::new(command.call_id(), Ok(()))))
                }
            }
        };
    }

    impl_unit_service!(TraceStart, RecordedMethod::Start);
    impl_unit_service!(TraceGroup, RecordedMethod::Group);
    impl_unit_service!(TraceGroupEnd, RecordedMethod::GroupEnd);
    impl_unit_service!(TraceStop, RecordedMethod::Stop);

    impl Service<Command<TraceStartChunk>> for RecordingChannel {
        type Response = CommandResult<TraceStartChunk>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<TraceStartChunk>) -> Self::Future<'_> {
            self.calls.lock().unwrap().push((
                RecordedMethod::StartChunk,
                command.call_id(),
                command.object_id(),
            ));
            ready(Ok(CommandResult::new(
                command.call_id(),
                Ok(TraceStartChunkResult::new(1)),
            )))
        }
    }

    impl Service<Command<TraceStopChunk>> for RecordingChannel {
        type Response = CommandResult<TraceStopChunk>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<TraceStopChunk>) -> Self::Future<'_> {
            self.calls.lock().unwrap().push((
                RecordedMethod::StopChunk,
                command.call_id(),
                command.object_id(),
            ));
            ready(Ok(CommandResult::new(
                command.call_id(),
                Ok(TraceStopChunkResult::new(1, 2, 32)),
            )))
        }
    }

    #[tokio::test]
    async fn tracing_facade_dispatches_six_ordered_context_commands() {
        // Arrange
        let calls = Arc::new(Mutex::new(Vec::new()));
        let client = Client::new(
            RecordingChannel {
                calls: Arc::clone(&calls),
            },
            AtomicCallIdSource::new(20),
        );
        let browser = client
            .browser(RemoteInitializer::new(
                ObjectId::new(1),
                None,
                BrowserKind::OBJECT_KIND,
            ))
            .unwrap();
        let context = browser
            .context(RemoteInitializer::new(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                BrowserContextKind::OBJECT_KIND,
            ))
            .unwrap();
        let tracing = context.tracing();

        // Act
        tracing.start(TraceStartOptions::default()).await.unwrap();
        let chunk = tracing
            .start_chunk(TraceStartChunkOptions::default())
            .await
            .unwrap();
        tracing
            .group(TraceGroupOptions::new("checkout", None))
            .await
            .unwrap();
        tracing.group_end().await.unwrap();
        let summary = tracing
            .stop_chunk(TraceStopChunkOptions::new(TraceStopChunkMode::Sink))
            .await
            .unwrap();
        tracing.stop().await.unwrap();

        // Assert
        assert_eq!(chunk.chunk_id(), 1);
        assert_eq!(summary.payload_bytes(), 32);
        assert_eq!(
            calls.lock().unwrap().as_slice(),
            &[
                (RecordedMethod::Start, CallId::new(20), ObjectId::new(2)),
                (
                    RecordedMethod::StartChunk,
                    CallId::new(21),
                    ObjectId::new(2)
                ),
                (RecordedMethod::Group, CallId::new(22), ObjectId::new(2)),
                (RecordedMethod::GroupEnd, CallId::new(23), ObjectId::new(2)),
                (RecordedMethod::StopChunk, CallId::new(24), ObjectId::new(2)),
                (RecordedMethod::Stop, CallId::new(25), ObjectId::new(2)),
            ]
        );
    }
}
