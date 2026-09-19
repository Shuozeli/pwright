use super::*;
use pwright_backend_cdp_input::CdpInputPolicy;
use pwright_core::{CallId, CommandSpec, ObjectId, ResourceClass, ResourceLimits, ResourceScope};
use pwright_fake::FakeCdpClient;
use std::{
    num::NonZeroUsize,
    sync::Mutex,
    task::{Context, Waker},
};

#[derive(Default)]
struct RecordingDelay(Mutex<Vec<Duration>>);
impl CdpKeyboardDelay for RecordingDelay {
    fn wait(
        &self,
        duration: Duration,
    ) -> Pin<Box<dyn Future<Output = Result<(), CdpError>> + Send + '_>> {
        self.0.lock().unwrap().push(duration);
        Box::pin(std::future::ready(Ok(())))
    }
}
struct PendingDelay;
impl CdpKeyboardDelay for PendingDelay {
    fn wait(&self, _: Duration) -> Pin<Box<dyn Future<Output = Result<(), CdpError>> + Send + '_>> {
        Box::pin(std::future::pending())
    }
}
struct ClosedDelay;
impl CdpKeyboardDelay for ClosedDelay {
    fn wait(&self, _: Duration) -> Pin<Box<dyn Future<Output = Result<(), CdpError>> + Send + '_>> {
        Box::pin(std::future::ready(Err(CdpError::Closed)))
    }
}
fn profile(bytes: usize, count: usize) -> CdpKeyboardProductionProfile {
    CdpKeyboardProductionProfile::new(
        ResourceScope::extension(13, 31),
        bytes,
        NonZeroUsize::new(count).unwrap(),
    )
    .unwrap()
}
fn factory(
    profile: CdpKeyboardProductionProfile,
    delay: Arc<dyn CdpKeyboardDelay>,
) -> (CdpKeyboardServiceFactory, ProductionResourceLedger) {
    let requirements = profile.requirements();
    let bytes = requirements.class(ResourceClass::BackendStateBytes);
    let limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
        ResourceClass::BackendStateBytes,
        bytes.global,
        bytes.per_scope,
    );
    let resources = ProductionResourceLedger::new(limits, requirements).unwrap();
    let factory = CdpKeyboardServiceFactory::new_with_production_resources(
        profile,
        &resources,
        delay,
        KeyboardPlatform::Other,
    )
    .unwrap();
    (factory, resources)
}
fn key(value: &str) -> DownParams {
    DownParams {
        key: KeyboardKey::copy_from_str(value).unwrap(),
    }
}
fn up(value: &str) -> UpParams {
    UpParams {
        key: key(value).key,
    }
}
fn press(value: &str) -> PressParams {
    PressParams {
        key: key(value).key,
        delay: None,
    }
}
fn page(
    factory: &CdpKeyboardServiceFactory,
) -> (CdpKeyboardServices, Arc<FakeCdpClient>, CdpInputCoordinator) {
    let client = Arc::new(FakeCdpClient::from_html(""));
    let input = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
    (factory.bind(client.clone(), input.clone()), client, input)
}

#[test]
fn operation_is_admitted_before_poll_and_released_when_unpolled_future_drops() {
    // Arrange
    let (factory, resources) = factory(profile(128, 1), Arc::new(RecordingDelay::default()));
    let (page, client, _) = page(&factory);
    // Act
    let future = page.keyboard_down.call(key("Shift"));
    // Assert
    assert_eq!(factory.active_operations(), 1);
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
    assert!(client.calls().is_empty());
    drop(future);
    assert_eq!(factory.active_operations(), 0);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn admission_limit_is_shared_across_pages_and_command_types() {
    // Arrange
    let (factory, resources) = factory(profile(128, 1), Arc::new(RecordingDelay::default()));
    let (first, _, _) = page(&factory);
    let (second, client, _) = page(&factory);
    let held = first.keyboard_down.call(key("Shift"));
    // Act
    let error = second.keyboard_press.call(press("a")).await.unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    assert!(client.calls().is_empty());
    assert_eq!(factory.active_operations(), 1);
    drop(held);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn argument_capacity_is_rejected_before_native_input() {
    // Arrange
    let (factory, resources) = factory(profile(4, 1), Arc::new(RecordingDelay::default()));
    let (page, client, _) = page(&factory);
    let mut owned = String::with_capacity(128);
    owned.push('a');
    // Act
    let error = page
        .keyboard_down
        .call(DownParams {
            key: KeyboardKey::try_from(owned).unwrap(),
        })
        .await
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    assert!(client.calls().is_empty());
    assert_eq!(factory.active_operations(), 0);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn failed_ledger_reservation_rolls_back_operation_count() {
    // Arrange
    let selected = profile(128, 1);
    let (factory, resources) = factory(selected, Arc::new(RecordingDelay::default()));
    let ledger = resources.claim(selected.requirements()).unwrap();
    let held = ledger
        .reserve(selected.scope(), ResourceClass::BackendStateBytes, 1)
        .unwrap();
    let (page, client, _) = page(&factory);
    // Act
    let error = page.keyboard_down.call(key("a")).await.unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    assert_eq!(factory.active_operations(), 0);
    assert!(client.calls().is_empty());
    drop(held);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn all_services_share_injected_page_state_with_other_input_consumers() {
    // Arrange
    let (factory, _) = factory(profile(128, 1), Arc::new(RecordingDelay::default()));
    let (page, client, input) = page(&factory);
    assert!(page.keyboard_down.shares_input_state_with(&input));
    assert!(page.keyboard_up.shares_input_state_with(&input));
    assert!(page.keyboard_press.shares_input_state_with(&input));
    assert!(page.keyboard_insert_text.shares_input_state_with(&input));
    assert!(page.keyboard_type.shares_input_state_with(&input));
    page.keyboard_down.call(key("Shift")).await.unwrap();
    let mut other = input.begin_transaction(client.clone(), None).await.unwrap();
    // Act
    other.key_down_named("KeyA", false).await.unwrap();
    // Assert
    let calls = client.calls();
    assert_eq!(calls[1].1[0]["key"], "A");
    assert_eq!(calls[1].1[0]["modifiers"], 8);
    other.key_up_named("KeyA", false).await.unwrap();
    other.finish().await.unwrap();
    page.keyboard_up.call(up("Shift")).await.unwrap();
}

#[tokio::test]
async fn unknown_tail_retains_acknowledged_prefix_in_page_state() {
    // Arrange
    let (factory, _) = factory(profile(128, 1), Arc::new(RecordingDelay::default()));
    let (page, client, _) = page(&factory);
    // Act
    let error = page.keyboard_press.call(press("Shift+")).await.unwrap_err();
    // Assert
    assert_eq!(error.message(), "Unknown key: \"\"");
    assert!(matches!(
        error.details(),
        Some(KeyboardErrorDetails::InvalidKey(_))
    ));
    assert_eq!(client.calls().len(), 1);
    page.keyboard_press.call(press("KeyB")).await.unwrap();
    assert_eq!(client.calls()[1].1[0]["key"], "B");
    page.keyboard_up.call(up("Shift")).await.unwrap();
}

#[tokio::test]
async fn press_uses_one_injected_delay_and_reverse_release_order() {
    // Arrange
    let delay = Arc::new(RecordingDelay::default());
    let (factory, _) = factory(profile(128, 1), delay.clone());
    let (page, client, _) = page(&factory);
    let mut params = press("Control+Shift+KeyA");
    params.delay = Some(Duration::from_millis(5));
    // Act
    page.keyboard_press.call(params).await.unwrap();
    // Assert
    assert_eq!(*delay.0.lock().unwrap(), [Duration::from_millis(5)]);
    let calls = client.calls();
    let keys: Vec<_> = calls
        .iter()
        .map(|call| call.1[0]["key"].as_str().unwrap())
        .collect();
    assert_eq!(keys, ["Control", "Shift", "A", "A", "Shift", "Control"]);
    assert!(calls.iter().all(|call| call.0 == "Input.dispatchKeyEvent"));
}

#[tokio::test]
async fn insert_text_is_one_native_call_without_key_events_or_delay() {
    // Arrange
    let delay = Arc::new(RecordingDelay::default());
    let (factory, _) = factory(profile(128, 1), delay.clone());
    let (page, client, _) = page(&factory);
    // Act
    page.keyboard_insert_text
        .call(InsertTextParams {
            text: KeyboardText::copy_from_str("你好").unwrap(),
        })
        .await
        .unwrap();
    // Assert
    let calls = client.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "Input.insertText");
    assert_eq!(calls[0].1[0], "你好");
    assert!(delay.0.lock().unwrap().is_empty());
}

#[tokio::test]
async fn type_text_delays_per_character_and_uses_unicode_insertion() {
    // Arrange
    let delay = Arc::new(RecordingDelay::default());
    let (factory, _) = factory(profile(128, 1), delay.clone());
    let (page, client, _) = page(&factory);
    // Act
    page.keyboard_type
        .call(TypeParams {
            text: KeyboardText::copy_from_str("a你").unwrap(),
            delay: Some(Duration::from_millis(7)),
        })
        .await
        .unwrap();
    // Assert
    assert_eq!(*delay.0.lock().unwrap(), [Duration::from_millis(7); 2]);
    let methods: Vec<_> = client.calls().into_iter().map(|call| call.0).collect();
    assert_eq!(
        methods,
        [
            "Input.dispatchKeyEvent",
            "Input.dispatchKeyEvent",
            "Input.insertText"
        ]
    );
}

#[tokio::test]
async fn zero_delay_does_not_invoke_clock() {
    // Arrange
    let delay = Arc::new(RecordingDelay::default());
    let (factory, _) = factory(profile(128, 1), delay.clone());
    let (page, _, _) = page(&factory);
    let mut params = press("a");
    params.delay = Some(Duration::ZERO);
    // Act
    page.keyboard_press.call(params).await.unwrap();
    // Assert
    assert!(delay.0.lock().unwrap().is_empty());
}

#[tokio::test]
async fn cancelled_interleaved_press_releases_admission_but_not_page_keys() {
    // Arrange
    let (factory, resources) = factory(profile(128, 1), Arc::new(PendingDelay));
    let (page, client, _) = page(&factory);
    let mut params = press("Shift");
    params.delay = Some(Duration::from_secs(1));
    let mut future = page.keyboard_press.call(params);
    assert!(
        future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    drop(future);
    let context = OperationContext::new(
        CallId::new(1),
        ObjectId::new(1),
        KeyboardPress::METHOD,
        None,
    );
    // Act
    page.keyboard_press
        .cancel(context, CancellationReason::DeadlineExceeded)
        .await
        .unwrap();
    // Assert
    assert_eq!(factory.active_operations(), 0);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
    assert_eq!(client.calls().len(), 1);
    page.keyboard_press.call(press("KeyA")).await.unwrap();
    assert_eq!(client.calls()[1].1[0]["key"], "A");
    page.keyboard_up.call(up("Shift")).await.unwrap();
}

#[tokio::test]
async fn exclusive_policy_cancellation_recovers_keys() {
    // Arrange
    let (factory, resources) = factory(profile(128, 1), Arc::new(PendingDelay));
    let client = Arc::new(FakeCdpClient::from_html(""));
    let page = factory.bind(client.clone(), CdpInputCoordinator::new());
    let mut params = press("Shift");
    params.delay = Some(Duration::from_secs(1));
    let mut future = page.keyboard_press.call(params);
    assert!(
        future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    drop(future);
    let context = OperationContext::new(
        CallId::new(1),
        ObjectId::new(1),
        KeyboardPress::METHOD,
        None,
    );
    // Act
    page.keyboard_press
        .cancel(context, CancellationReason::DeadlineExceeded)
        .await
        .unwrap();
    // Assert
    assert_eq!(client.calls().len(), 2);
    assert_eq!(client.calls()[1].1[0]["type"], "keyUp");
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn exclusive_finish_failure_is_classified_as_cleanup() {
    // Arrange
    let (factory, resources) = factory(profile(128, 1), Arc::new(RecordingDelay::default()));
    let client = Arc::new(FakeCdpClient::from_html(""));
    client.set_input_errors([None, Some("key up failed".into())]);
    let page = factory.bind(client.clone(), CdpInputCoordinator::new());
    // Act
    let error = page.keyboard_down.call(key("Shift")).await.unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::Cleanup);
    assert!(error.message().contains("key up failed"));
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn oversized_native_diagnostic_returns_explicit_resource_error() {
    // Arrange
    let (factory, resources) = factory(profile(128, 1), Arc::new(RecordingDelay::default()));
    let (page, client, _) = page(&factory);
    client.set_input_errors([Some("x".repeat(5000))]);
    // Act
    let error = page.keyboard_down.call(key("a")).await.unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    assert!(error.message().contains("4096-byte"));
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn profile_rejects_oversized_arguments_and_overflowing_total_budget() {
    // Arrange
    let scope = ResourceScope::extension(13, 31);
    // Act
    let oversized = CdpKeyboardProductionProfile::new(
        scope,
        KEYBOARD_TEXT_MAX_BYTES + 1,
        NonZeroUsize::new(1).unwrap(),
    );
    let overflow =
        CdpKeyboardProductionProfile::new(scope, 128, NonZeroUsize::new(usize::MAX).unwrap());
    // Assert
    assert_eq!(
        oversized,
        Err(CdpKeyboardProfileError::ArgumentLimitTooLarge)
    );
    assert_eq!(overflow, Err(CdpKeyboardProfileError::ByteBudgetOverflow));
}

#[tokio::test]
async fn native_delay_failure_preserves_category_and_interleaved_prefix() {
    // Arrange
    let (factory, resources) = factory(profile(128, 1), Arc::new(ClosedDelay));
    let (page, client, _) = page(&factory);
    let mut params = press("Shift");
    params.delay = Some(Duration::from_millis(1));
    // Act
    let error = page.keyboard_press.call(params).await.unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::TransportClosed);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
    page.keyboard_press.call(press("KeyA")).await.unwrap();
    assert_eq!(client.calls()[1].1[0]["key"], "A");
    page.keyboard_up.call(up("Shift")).await.unwrap();
}

#[tokio::test]
async fn overlong_chord_rejects_before_any_key_event() {
    // Arrange
    let (factory, resources) = factory(profile(128, 1), Arc::new(RecordingDelay::default()));
    let (page, client, _) = page(&factory);
    let chord = std::iter::repeat_n("a", 17).collect::<Vec<_>>().join("+");
    // Act
    let error = page.keyboard_press.call(press(&chord)).await.unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    assert!(client.calls().is_empty());
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn factory_rejects_undeclared_production_requirements() {
    // Arrange
    let selected = profile(128, 1);
    let required = selected.requirements();
    let bytes = required.class(ResourceClass::BackendStateBytes);
    let limits = ResourceLimits::deny_all(required.scopes, required.leases).with_class(
        ResourceClass::BackendStateBytes,
        bytes.global,
        bytes.per_scope,
    );
    let resources =
        ProductionResourceLedger::new(limits, pwright_core::ResourceRequirements::none()).unwrap();
    // Act
    let result = CdpKeyboardServiceFactory::new_with_production_resources(
        selected,
        &resources,
        Arc::new(RecordingDelay::default()),
        KeyboardPlatform::Other,
    );
    // Assert
    assert!(result.is_err());
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}
