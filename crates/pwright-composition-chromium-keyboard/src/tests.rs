use super::*;
use pwright_backend_cdp_input::CdpInputPolicy;
use pwright_cdp::connection::CdpError;
use pwright_core::{CallId, CallMetadata, CommandSpec, ResourceLimits};
use pwright_fake::FakeCdpClient;
use pwright_server_wire::{ConcurrentServerFrameHandler, ServerFrameDisposition};
use pwright_transport::{FrameTransport, WireFrame};
use std::{
    future::{Ready, ready},
    num::NonZeroUsize,
    sync::Mutex,
    task::{Context, Waker},
    time::Duration,
};

#[cfg(feature = "runtime")]
mod controlled;

struct Delay;
impl CdpKeyboardDelay for Delay {
    fn wait(&self, _: Duration) -> Pin<Box<dyn Future<Output = Result<(), CdpError>> + Send + '_>> {
        Box::pin(std::future::pending())
    }
}
fn profile() -> ChromiumKeyboardProfile {
    ChromiumKeyboardProfile::new(
        CdpKeyboardProductionProfile::new(
            ResourceScope::extension(13, 1),
            128,
            NonZeroUsize::new(1).unwrap(),
        )
        .unwrap(),
        KeyboardCodecLimits::new(4096),
        ProtocolVersion::new(1, 0),
        FrameLimits::new(16384, 0, 0, 16384),
        FrameLimits::new(16384, 0, 0, 16384),
    )
}
fn resources(required: ResourceRequirements) -> ProductionResourceLedger {
    let mut limits = ResourceLimits::deny_all(required.scopes, required.leases);
    for class in ResourceClass::ALL {
        let bytes = required.class(class);
        limits = limits.with_class(class, bytes.global, bytes.per_scope);
    }
    ProductionResourceLedger::new(limits, required).unwrap()
}
fn installation(
    policy: CdpInputPolicy,
) -> (
    ChromiumKeyboardInstallation,
    Arc<FakeCdpClient>,
    ProductionResourceLedger,
) {
    let profile = profile();
    let resources = resources(profile.requirements());
    let client = Arc::new(FakeCdpClient::from_html(""));
    let installed = ChromiumKeyboardInstallation::install(
        ObjectId::new(7),
        client.clone(),
        CdpInputCoordinator::with_policy(policy),
        profile,
        Arc::new(Delay),
        KeyboardPlatform::Other,
        &resources,
    )
    .unwrap();
    (installed, client, resources)
}
fn key(value: &str) -> DownParams {
    DownParams {
        key: KeyboardKey::copy_from_str(value).unwrap(),
    }
}
fn down(page: u64) -> Command<KeyboardDown> {
    Command::new(
        CallId::new(31),
        ObjectId::new(page),
        CallMetadata::default(),
        key("Shift"),
    )
}
fn press() -> Command<KeyboardPress> {
    Command::new(
        CallId::new(32),
        ObjectId::new(7),
        CallMetadata::default(),
        PressParams {
            key: key("Shift").key,
            delay: Some(Duration::from_secs(1)),
        },
    )
}

#[tokio::test]
async fn wrong_page_rejects_before_backend_admission_or_cdp() {
    // Arrange
    let (installed, client, resources) = installation(CdpInputPolicy::Interleaved);
    let page = installed.page();
    // Act
    let future = page.call(down(8));
    // Assert
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
    let reply = future.await.unwrap();
    assert_eq!(reply.call_id(), CallId::new(31));
    assert_eq!(
        reply.into_outcome().unwrap_err().kind(),
        FailureKind::ProtocolViolation
    );
    assert!(client.calls().is_empty());
}

#[test]
fn escaped_page_keeps_route_lease_after_installation_drops() {
    // Arrange
    let (installed, _, resources) = installation(CdpInputPolicy::Interleaved);
    let page = installed.page();
    // Act
    drop(installed);
    // Assert
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current,
        ChromiumKeyboardPage::route_requirements()
            .class(ResourceClass::BackendStateBytes)
            .global
    );
    drop(page);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn route_preserves_eager_backend_admission_before_polling() {
    // Arrange
    let (installed, client, resources) = installation(CdpInputPolicy::Interleaved);
    let page = installed.page();
    // Act
    let future = page.call(down(7));
    // Assert
    assert_eq!(resources.metrics().unwrap().active_leases, 2);
    assert!(client.calls().is_empty());
    drop(future);
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
}

#[tokio::test]
async fn revoked_route_rejects_a_previously_admitted_unpolled_call() {
    // Arrange
    let (installed, client, resources) = installation(CdpInputPolicy::Interleaved);
    let page = installed.page();
    let future = page.call(down(7));
    page.revoke_admission();
    // Act
    let error = future.await.unwrap().into_outcome().unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::TargetClosed);
    assert!(client.calls().is_empty());
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
}

#[tokio::test]
async fn revoked_route_rejects_new_calls_without_reserving_an_operation() {
    // Arrange
    let (installed, client, resources) = installation(CdpInputPolicy::Interleaved);
    let page = installed.page();
    page.revoke_admission();
    // Act
    let future = page.call(down(7));
    // Assert
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
    assert_eq!(
        future.await.unwrap().into_outcome().unwrap_err().kind(),
        FailureKind::TargetClosed
    );
    assert!(client.calls().is_empty());
}

#[test]
fn revocation_does_not_claim_cancellation_of_a_started_operation() {
    // Arrange
    let (installed, client, resources) = installation(CdpInputPolicy::Interleaved);
    let page = installed.page();
    let mut future = page.call(press());
    assert!(
        future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    // Act
    page.revoke_admission();
    // Assert
    assert!(
        future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    assert_eq!(client.calls().len(), 1);
    assert_eq!(resources.metrics().unwrap().active_leases, 2);
    drop(future);
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
}

#[tokio::test]
async fn cancellation_cleanup_remains_addressable_after_revocation() {
    // Arrange
    let (installed, client, resources) = installation(CdpInputPolicy::Exclusive);
    let page = installed.page();
    let mut future = page.call(press());
    assert!(
        future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    page.revoke_admission();
    drop(future);
    let context = OperationContext::new(
        CallId::new(32),
        ObjectId::new(7),
        KeyboardPress::METHOD,
        None,
    );
    // Act
    <ChromiumKeyboardPage as CommandCancellation<KeyboardPress>>::cancel(
        &page,
        context,
        CancellationReason::DeadlineExceeded,
    )
    .await
    .unwrap();
    // Assert
    assert_eq!(client.calls().len(), 2);
    assert_eq!(client.calls()[1].1[0]["type"], "keyUp");
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
}

#[tokio::test]
async fn cancellation_for_wrong_page_does_not_touch_shared_input() {
    // Arrange
    let (installed, client, _) = installation(CdpInputPolicy::Exclusive);
    let page = installed.page();
    let mut future = page.call(press());
    assert!(
        future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    drop(future);
    let context = OperationContext::new(
        CallId::new(32),
        ObjectId::new(8),
        KeyboardPress::METHOD,
        None,
    );
    // Act
    let error = <ChromiumKeyboardPage as CommandCancellation<KeyboardPress>>::cancel(
        &page,
        context,
        CancellationReason::DeadlineExceeded,
    )
    .await
    .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::ProtocolViolation);
    assert_eq!(client.calls().len(), 1);
}

#[tokio::test]
async fn multiple_page_routes_reuse_a_single_factory_admission_budget() {
    // Arrange
    let selected = profile().backend();
    let required = selected
        .requirements()
        .combine_concurrent(ChromiumKeyboardPage::route_requirements())
        .combine_concurrent(ChromiumKeyboardPage::route_requirements());
    let resources = resources(required);
    let factory = CdpKeyboardServiceFactory::new_with_production_resources(
        selected,
        &resources,
        Arc::new(Delay),
        KeyboardPlatform::Other,
    )
    .unwrap();
    let first = ChromiumKeyboardPage::bind(
        ObjectId::new(7),
        Arc::new(FakeCdpClient::from_html("")),
        CdpInputCoordinator::new(),
        &factory,
        &resources,
    )
    .unwrap();
    let client = Arc::new(FakeCdpClient::from_html(""));
    let second = ChromiumKeyboardPage::bind(
        ObjectId::new(8),
        client.clone(),
        CdpInputCoordinator::new(),
        &factory,
        &resources,
    )
    .unwrap();
    let held = first.call(down(7));
    // Act
    let error = second
        .call(down(8))
        .await
        .unwrap()
        .into_outcome()
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    assert!(client.calls().is_empty());
    assert_eq!(factory.active_operations(), 1);
    drop(held);
    drop(first);
    drop(second);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn undeclared_route_resources_fail_before_binding_or_cdp() {
    // Arrange
    let selected = profile().backend();
    let resources = resources(selected.requirements());
    let factory = CdpKeyboardServiceFactory::new_with_production_resources(
        selected,
        &resources,
        Arc::new(Delay),
        KeyboardPlatform::Other,
    )
    .unwrap();
    let client = Arc::new(FakeCdpClient::from_html(""));
    // Act
    let result = ChromiumKeyboardPage::bind(
        ObjectId::new(7),
        client.clone(),
        CdpInputCoordinator::new(),
        &factory,
        &resources,
    );
    // Assert
    assert!(result.is_err());
    assert!(client.calls().is_empty());
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[derive(Default)]
struct Transport(Mutex<Option<WireFrame>>);
impl FrameTransport for Transport {
    type Error = Infallible;
    type Send<'a> = Ready<Result<(), Infallible>>;
    type Receive<'a> = Ready<Result<Option<WireFrame>, Infallible>>;
    fn send(&self, frame: WireFrame) -> Self::Send<'_> {
        assert!(self.0.lock().unwrap().replace(frame).is_none());
        ready(Ok(()))
    }
    fn receive(&self) -> Self::Receive<'_> {
        ready(Ok(self.0.lock().unwrap().take()))
    }
}

#[tokio::test]
async fn generated_wire_chain_executes_all_five_commands_on_bound_page() {
    // Arrange
    let (installed, client, resources) = installation(CdpInputPolicy::Interleaved);
    let protocol = installed.protocol();
    let handlers = installed.handlers();
    let transport = Arc::new(Transport::default());
    // Act: independent encode/dispatch/decode transactions share the bound Page.
    macro_rules! roundtrip {
        ($command:ty, $params:expr, $id:expr) => {{
            let frame = protocol
                .encode(&Command::<$command>::new(
                    CallId::new($id),
                    ObjectId::new(7),
                    CallMetadata::default(),
                    $params,
                ))
                .unwrap();
            let disposition = handlers
                .handle_concurrently(frame, transport.clone())
                .await
                .unwrap();
            // Assert
            assert!(matches!(disposition, ServerFrameDisposition::Handled(_)));
            let reply = transport.0.lock().unwrap().take().unwrap();
            let result = protocol.decode::<CommandResult<$command>>(reply).unwrap();
            assert_eq!(result.call_id(), CallId::new($id));
            assert_eq!(result.into_outcome(), Ok(()));
            assert_eq!(resources.metrics().unwrap().active_leases, 1);
        }};
    }
    roundtrip!(KeyboardDown, key("Shift"), 1);
    roundtrip!(
        KeyboardPress,
        PressParams {
            key: key("KeyA").key,
            delay: None
        },
        2
    );
    roundtrip!(
        KeyboardUp,
        UpParams {
            key: key("Shift").key
        },
        3
    );
    roundtrip!(
        KeyboardInsertText,
        InsertTextParams {
            text: KeyboardText::copy_from_str("你好").unwrap()
        },
        4
    );
    roundtrip!(
        KeyboardType,
        TypeParams {
            text: KeyboardText::copy_from_str("b").unwrap(),
            delay: None
        },
        5
    );
    assert_eq!(client.calls()[1].1[0]["key"], "A");
    assert_eq!(client.calls().len(), 7);
    drop(handlers);
    drop(installed);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}
