//! Exact route identity must survive runtime cancellation and queued cleanup.
use super::*;
use pwright_browser_model::{
    BROWSER_CONTEXT_OBJECT_KIND, BROWSER_OBJECT_KIND, FRAME_OBJECT_KIND, PAGE_OBJECT_KIND,
};
use pwright_core::ObjectKindCatalog;
use pwright_runtime::{CleanupQueue, ControlledDispatcher, DeadlineScheduler, RuntimeLimits};
use std::task::Poll;

struct ManualDeadline(Arc<AtomicBool>);
impl DeadlineScheduler for ManualDeadline {
    type Sleep<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
    fn sleep(&self, _: Duration) -> Option<Self::Sleep<'_>> {
        Some(Box::pin(std::future::poll_fn(|_| {
            if self.0.load(Ordering::Acquire) {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })))
    }
}
type Runtime = ControlledDispatcher<(), (), ManualDeadline>;
struct Fixture {
    runtime: Runtime,
    page: Arc<ChromiumKeyboardPage>,
    client: Arc<FakeCdpClient>,
    resources: ProductionResourceLedger,
    expired: Arc<AtomicBool>,
}
impl Fixture {
    fn new(policy: CdpInputPolicy) -> Self {
        let limits = RuntimeLimits::new(3, 4).with_active_commands(1);
        let selected = profile();
        let resources = resources(
            selected
                .requirements()
                .combine_concurrent(limits.resource_requirements()),
        );
        let expired = Arc::new(AtomicBool::new(false));
        let runtime = ControlledDispatcher::with_cleanup_executor_and_object_kinds(
            (),
            (),
            ManualDeadline(expired.clone()),
            limits,
            CleanupQueue::new(4),
            ObjectKindCatalog::new(&[
                BROWSER_OBJECT_KIND,
                BROWSER_CONTEXT_OBJECT_KIND,
                PAGE_OBJECT_KIND,
                FRAME_OBJECT_KIND,
            ]),
        )
        .unwrap()
        .with_production_resources(&resources)
        .unwrap();
        for id in [7, 8] {
            runtime
                .register_object(ObjectId::new(id), None, PAGE_OBJECT_KIND.tag)
                .unwrap();
        }
        runtime
            .register_object(
                ObjectId::new(9),
                Some(ObjectId::new(7)),
                FRAME_OBJECT_KIND.tag,
            )
            .unwrap();
        let client = Arc::new(FakeCdpClient::from_html(""));
        let installation = ChromiumKeyboardInstallation::install(
            ObjectId::new(7),
            client.clone(),
            CdpInputCoordinator::with_policy(policy),
            selected,
            Arc::new(Delay),
            KeyboardPlatform::Other,
            &resources,
        )
        .unwrap();
        Self {
            runtime,
            page: installation.page(),
            client,
            resources,
            expired,
        }
    }
    fn assert_idle(&self) {
        assert_eq!(self.runtime.pending_call_count(), Ok(0));
        assert_eq!(
            self.runtime.connection_metrics().unwrap().active_commands,
            0
        );
        assert_eq!(self.runtime.scheduler_metrics().unwrap().active_targets, 0);
        assert_eq!(
            self.resources
                .metrics()
                .unwrap()
                .class(ResourceClass::ActiveCommand)
                .current,
            0
        );
    }
}
fn poll<T>(future: Pin<&mut impl Future<Output = T>>) -> Poll<T> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

#[tokio::test]
async fn down_and_up_dispatch_to_distinct_native_operations() {
    // Arrange
    let fixture = Fixture::new(CdpInputPolicy::Interleaved);
    fixture
        .runtime
        .dispatch_with(down(7), fixture.page.as_ref(), fixture.page.clone())
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    let up = Command::<KeyboardUp>::new(
        CallId::new(33),
        ObjectId::new(7),
        CallMetadata::default(),
        UpParams {
            key: key("Shift").key,
        },
    );
    // Act
    fixture
        .runtime
        .dispatch_with(up, fixture.page.as_ref(), fixture.page.clone())
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    // Assert
    let calls = fixture.client.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].1[0]["type"], "rawKeyDown");
    assert_eq!(calls[1].1[0]["type"], "keyUp");
    fixture.assert_idle();
}

#[tokio::test]
async fn live_but_unbound_page_is_rejected_before_native_admission() {
    // Arrange
    let fixture = Fixture::new(CdpInputPolicy::Interleaved);
    // Act
    let error = fixture
        .runtime
        .dispatch_with(down(8), fixture.page.as_ref(), fixture.page.clone())
        .await
        .unwrap()
        .into_outcome()
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::ProtocolViolation);
    assert!(fixture.client.calls().is_empty());
    fixture.assert_idle();
}

#[tokio::test]
async fn frame_target_is_rejected_before_native_admission() {
    // Arrange
    let fixture = Fixture::new(CdpInputPolicy::Interleaved);
    // Act
    let error = fixture
        .runtime
        .dispatch_with(down(9), fixture.page.as_ref(), fixture.page.clone())
        .await
        .unwrap()
        .into_outcome()
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::ProtocolViolation);
    assert!(fixture.client.calls().is_empty());
    fixture.assert_idle();
}

#[test]
fn disposal_cancels_started_press_on_original_revoked_route() {
    // Arrange
    let fixture = Fixture::new(CdpInputPolicy::Exclusive);
    let mut operation = Box::pin(fixture.runtime.dispatch_with(
        press(),
        fixture.page.as_ref(),
        fixture.page.clone(),
    ));
    assert!(poll(operation.as_mut()).is_pending());
    fixture.page.revoke_admission();
    // Act
    fixture.runtime.dispose_object(ObjectId::new(7)).unwrap();
    let result = poll(operation.as_mut());
    // Assert
    let Poll::Ready(Ok(reply)) = result else {
        panic!("disposal must settle the operation")
    };
    assert_eq!(
        reply.into_outcome().unwrap_err().kind(),
        FailureKind::TargetClosed
    );
    let calls = fixture.client.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[1].1[0]["type"], "keyUp");
    fixture.assert_idle();
}

#[test]
fn deadline_uses_the_bound_pages_explicit_cleanup_policy() {
    for (policy, native_calls) in [
        (CdpInputPolicy::Exclusive, 2),
        (CdpInputPolicy::Interleaved, 1),
    ] {
        // Arrange
        let fixture = Fixture::new(policy);
        let (id, target, _, params) = press().into_parts();
        let command = Command::<KeyboardPress>::new(
            id,
            target,
            CallMetadata::new(Some(Duration::from_secs(1)), None),
            params,
        );
        let mut operation = Box::pin(fixture.runtime.dispatch_with(
            command,
            fixture.page.as_ref(),
            fixture.page.clone(),
        ));
        assert!(poll(operation.as_mut()).is_pending());
        // Act
        fixture.expired.store(true, Ordering::Release);
        let result = poll(operation.as_mut());
        // Assert
        let Poll::Ready(Ok(reply)) = result else {
            panic!("deadline must settle the operation")
        };
        assert_eq!(
            reply.into_outcome().unwrap_err().kind(),
            FailureKind::Timeout
        );
        assert_eq!(fixture.client.calls().len(), native_calls);
        if policy == CdpInputPolicy::Exclusive {
            assert_eq!(fixture.client.calls()[1].1[0]["type"], "keyUp");
        }
        fixture.assert_idle();
    }
}

#[test]
fn queued_command_checks_revocation_when_its_permit_arrives() {
    // Arrange
    let fixture = Fixture::new(CdpInputPolicy::Exclusive);
    let mut first = Box::pin(fixture.runtime.dispatch_with(
        press(),
        fixture.page.as_ref(),
        fixture.page.clone(),
    ));
    assert!(poll(first.as_mut()).is_pending());
    let mut queued = Box::pin(fixture.runtime.dispatch_with(
        down(7),
        fixture.page.as_ref(),
        fixture.page.clone(),
    ));
    assert!(poll(queued.as_mut()).is_pending());
    assert_eq!(fixture.client.calls().len(), 1);
    fixture.page.revoke_admission();
    drop(first);
    while let Some(mut job) = fixture.runtime.cleanup_executor().try_take().unwrap() {
        assert!(matches!(
            job.as_mut().poll(&mut Context::from_waker(Waker::noop())),
            Poll::Ready(Ok(()))
        ));
    }
    // Act
    let result = poll(queued.as_mut());
    // Assert
    let Poll::Ready(Ok(reply)) = result else {
        panic!("queued command must settle")
    };
    assert_eq!(
        reply.into_outcome().unwrap_err().kind(),
        FailureKind::TargetClosed
    );
    assert_eq!(fixture.client.calls().len(), 2);
    fixture.assert_idle();
}

#[test]
fn abandoned_cleanup_retains_exact_route_until_native_cleanup_finishes() {
    // Arrange
    let fixture = Fixture::new(CdpInputPolicy::Exclusive);
    let mut operation = Box::pin(fixture.runtime.dispatch_with(
        press(),
        fixture.page.as_ref(),
        fixture.page.clone(),
    ));
    assert!(poll(operation.as_mut()).is_pending());
    let weak = Arc::downgrade(&fixture.page);
    drop(operation);
    fixture.page.revoke_admission();
    drop(fixture.page);
    assert!(weak.upgrade().is_some());
    assert_eq!(fixture.client.calls().len(), 1);
    // Act
    while let Some(mut job) = fixture.runtime.cleanup_executor().try_take().unwrap() {
        assert!(matches!(
            job.as_mut().poll(&mut Context::from_waker(Waker::noop())),
            Poll::Ready(Ok(()))
        ));
    }
    // Assert
    assert!(weak.upgrade().is_none());
    assert_eq!(fixture.client.calls().len(), 2);
    assert_eq!(fixture.client.calls()[1].1[0]["type"], "keyUp");
    assert_eq!(
        fixture
            .resources
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current,
        0
    );
    drop(fixture.runtime);
    assert_eq!(fixture.resources.metrics().unwrap().active_leases, 0);
}
