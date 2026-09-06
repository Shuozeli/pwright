use std::future::Future;

use crate::{CancellationReason, CommandSpec, OperationContext, OperationError};

/// A typed capability operation.
///
/// The request, response, error, and future are associated at compile time.
/// The trait does not select an executor or require heap allocation.
pub trait Service<Request>: Send + Sync {
    type Response;
    type Error;
    type Future<'a>: Future<Output = Result<Self::Response, Self::Error>> + Send
    where
        Self: 'a;

    fn call(&self, request: Request) -> Self::Future<'_>;
}

/// Performs command-specific backend cleanup after cancellation wins.
///
/// This is separate from `Service` so cancellation policy remains composable
/// and so capability adapters can use the most precise primitive offered by a
/// backend. The returned future is awaited before the runtime reports the
/// command's terminal result.
pub trait CommandCancellation<C>: Send + Sync
where
    C: CommandSpec,
{
    type Future<'a>: Future<Output = Result<(), OperationError<C::ErrorDetails>>> + Send
    where
        Self: 'a;

    fn cancel(&self, context: OperationContext, reason: CancellationReason) -> Self::Future<'_>;
}

/// Wraps an inner service with additional behavior.
pub trait Layer<Inner> {
    type Service;

    fn layer(&self, inner: Inner) -> Self::Service;
}

/// A compile-time service composition stack.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Stack<S> {
    inner: S,
}

impl<S> Stack<S> {
    pub const fn new(inner: S) -> Self {
        Self { inner }
    }

    pub fn layer<L>(self, layer: L) -> Stack<L::Service>
    where
        L: Layer<S>,
    {
        Stack::new(layer.layer(self.inner))
    }

    pub fn into_inner(self) -> S {
        self.inner
    }

    pub const fn inner(&self) -> &S {
        &self.inner
    }
}

impl<Request, S> Service<Request> for Stack<S>
where
    S: Service<Request>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future<'a>
        = S::Future<'a>
    where
        Self: 'a;

    fn call(&self, request: Request) -> Self::Future<'_> {
        self.inner.call(request)
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::future::{Ready, ready};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{Layer, Service, Stack};

    struct RecordingService {
        last_request: Arc<AtomicUsize>,
    }

    impl Service<usize> for RecordingService {
        type Response = usize;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, request: usize) -> Self::Future<'_> {
            self.last_request.store(request, Ordering::SeqCst);
            ready(Ok(request + 1))
        }
    }

    struct CountCallsLayer {
        calls: Arc<AtomicUsize>,
    }

    struct CountCallsService<S> {
        inner: S,
        calls: Arc<AtomicUsize>,
    }

    impl<S> Layer<S> for CountCallsLayer {
        type Service = CountCallsService<S>;

        fn layer(&self, inner: S) -> Self::Service {
            CountCallsService {
                inner,
                calls: Arc::clone(&self.calls),
            }
        }
    }

    impl<Request, S> Service<Request> for CountCallsService<S>
    where
        S: Service<Request>,
    {
        type Response = S::Response;
        type Error = S::Error;
        type Future<'a>
            = S::Future<'a>
        where
            Self: 'a;

        fn call(&self, request: Request) -> Self::Future<'_> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.inner.call(request)
        }
    }

    #[test]
    fn stack_routes_a_request_through_the_outer_layer() {
        // Arrange
        let calls = Arc::new(AtomicUsize::new(0));
        let last_request = Arc::new(AtomicUsize::new(0));
        let service = Stack::new(RecordingService {
            last_request: Arc::clone(&last_request),
        })
        .layer(CountCallsLayer {
            calls: Arc::clone(&calls),
        });

        // Act
        let _response = service.call(41);

        // Assert
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(last_request.load(Ordering::SeqCst), 41);
    }
}
