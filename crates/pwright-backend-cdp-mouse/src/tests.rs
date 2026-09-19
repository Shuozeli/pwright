use super::*;
mod dispatch;
use pwright_core::{ResourceClass, ResourceLimits, ResourceScope};
use pwright_fake::FakeCdpClient;
use std::{
    future::{pending, ready},
    num::NonZeroUsize,
    sync::Mutex,
    task::{Context, Waker},
};

#[derive(Default)]
struct RecordingDelay(Mutex<Vec<Duration>>);
impl CdpMouseDelay for RecordingDelay {
    fn wait(self: Arc<Self>, duration: Duration) -> MouseNativeFuture {
        self.0.lock().unwrap().push(duration);
        Box::pin(ready(Ok(())))
    }
}
struct PendingDelay;
impl CdpMouseDelay for PendingDelay {
    fn wait(self: Arc<Self>, _: Duration) -> MouseNativeFuture {
        Box::pin(pending())
    }
}
fn number(value: f64) -> MouseNumber {
    MouseNumber::try_from(value).unwrap()
}
fn down() -> DownParams {
    DownParams {
        button: MouseButton::Left,
        click_count: 1,
    }
}
fn click() -> ClickParams {
    ClickParams {
        x: number(100.5),
        y: number(120.25),
        button: MouseButton::Left,
        click_count: MouseClickCount::ONE,
        delay: None,
    }
}
type Setup = (
    CdpMouseServiceFactory,
    ProductionResourceLedger,
    CdpMouseServices,
    Arc<FakeCdpClient>,
    CdpInputCoordinator,
);
fn setup(count: usize, delay: Arc<dyn CdpMouseDelay>) -> Setup {
    let client = Arc::new(FakeCdpClient::from_html(""));
    let input = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
    let bytes = CdpMouseServices::required_operation_bytes(client.clone(), &input).unwrap();
    let profile = CdpMouseProductionProfile::new(
        ResourceScope::extension(14, 32),
        bytes,
        NonZeroUsize::new(count).unwrap(),
    )
    .unwrap();
    let requirements = profile.requirements();
    let class = requirements.class(ResourceClass::BackendStateBytes);
    let limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
        ResourceClass::BackendStateBytes,
        class.global,
        class.per_scope,
    );
    let resources = ProductionResourceLedger::new(limits, requirements).unwrap();
    let factory =
        CdpMouseServiceFactory::new_with_production_resources(profile, &resources, delay).unwrap();
    let services = factory.bind(client.clone(), input.clone()).unwrap();
    (factory, resources, services, client, input)
}
#[tokio::test]
async fn all_six_starters_share_page_state_and_dispatch_native_events() {
    // Arrange
    let delay = Arc::new(RecordingDelay::default());
    let (factory, _, services, client, input) = setup(1, delay.clone());
    // Act
    let mut movement = services
        .mouse_move
        .start(MoveParams {
            x: number(20.0),
            y: number(30.0),
            steps: MouseMoveSteps::ONE,
        })
        .unwrap();
    movement.result().await.unwrap();
    movement.retire().await.unwrap();
    drop(movement);
    let mut press = services.mouse_down.start(down()).unwrap();
    press.result().await.unwrap();
    press.retire().await.unwrap();
    drop(press);
    let mut wheel = services
        .mouse_wheel
        .start(WheelParams {
            delta_x: number(1.5),
            delta_y: number(-2.5),
        })
        .unwrap();
    wheel.result().await.unwrap();
    wheel.retire().await.unwrap();
    drop(wheel);
    let mut release = services
        .mouse_up
        .start(UpParams {
            button: MouseButton::Left,
            click_count: 1,
        })
        .unwrap();
    release.result().await.unwrap();
    release.retire().await.unwrap();
    drop(release);
    let mut single = services.mouse_click.start(click()).unwrap();
    single.result().await.unwrap();
    single.retire().await.unwrap();
    drop(single);
    let mut double = services
        .mouse_dblclick
        .start(DblclickParams {
            x: number(150.0),
            y: number(160.0),
            button: MouseButton::Right,
            delay: Some(Duration::from_micros(500)),
        })
        .unwrap();
    double.result().await.unwrap();
    double.retire().await.unwrap();
    drop(double);
    // Assert
    assert!(services.mouse_click.shares_input_state_with(&input));
    assert!(services.mouse_down.shares_input_state_with(&input));
    let calls = client.calls();
    assert_eq!(calls.len(), 12);
    assert_eq!(calls[1].1[0]["x"], 20.0);
    assert_eq!(calls[1].1[0]["y"], 30.0);
    assert_eq!(calls[2].1[0]["type"], "mouseWheel");
    assert_eq!(calls[2].1[0]["deltaX"], 1.5);
    assert_eq!(calls[2].1[0]["deltaY"], -2.5);
    assert_eq!(calls[5].1[0]["x"], 100.5);
    assert_eq!(calls[10].1[0]["clickCount"], 2);
    assert_eq!(calls[10].1[0]["button"], "right");
    assert_eq!(*delay.0.lock().unwrap(), [Duration::from_micros(500); 3]);
    assert_eq!(factory.active_operations(), 0);
}
#[tokio::test]
async fn eager_admission_is_retained_through_result_and_retirement_until_owner_drop() {
    // Arrange
    let (factory, resources, services, client, _) = setup(1, Arc::new(RecordingDelay::default()));
    // Act
    let mut operation = services.mouse_down.start(down()).unwrap();
    // Assert
    assert_eq!(factory.active_operations(), 1);
    assert!(client.calls().is_empty());
    assert!(services.mouse_down.start(down()).is_err());
    operation.result().await.unwrap();
    operation.retire().await.unwrap();
    assert_eq!(factory.active_operations(), 1);
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
    drop(operation);
    assert_eq!(factory.active_operations(), 0);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}
#[tokio::test]
async fn abandoning_result_polling_keeps_native_reply_and_admission_for_retirement() {
    // Arrange
    let (factory, _, services, client, _) = setup(1, Arc::new(RecordingDelay::default()));
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_pointer_gates([Some(gate.clone())]);
    let mut operation = services.mouse_down.start(down()).unwrap();
    let mut result = Box::pin(operation.result());
    assert!(
        result
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    // Act
    drop(result);
    // Assert
    assert_eq!(operation.pending_native_calls(), 1);
    assert_eq!(factory.active_operations(), 1);
    let mut retirement = Box::pin(operation.retire());
    assert!(
        retirement
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    drop(retirement);
    gate.notify_one();
    operation.retire().await.unwrap();
    assert_eq!(operation.pending_native_calls(), 0);
    assert_eq!(client.calls().len(), 1);
}
#[tokio::test]
async fn early_click_error_keeps_later_native_tail_after_result() {
    // Arrange
    let (factory, _, services, client, _) = setup(1, Arc::new(RecordingDelay::default()));
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_input_errors([Some("move failed".into()), None, None]);
    client.set_pointer_gates([None, Some(gate.clone()), None]);
    let mut operation = services.mouse_click.start(click()).unwrap();
    // Act
    let error = operation.result().await.unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::Backend);
    assert_eq!(client.calls().len(), 3);
    assert_eq!(operation.pending_native_calls(), 1);
    assert_eq!(factory.active_operations(), 1);
    let mut retirement = Box::pin(operation.retire());
    assert!(
        retirement
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    gate.notify_one();
    retirement.await.unwrap();
    assert_eq!(client.calls().len(), 3);
    assert_eq!(operation.pending_native_calls(), 0);
}
#[tokio::test]
async fn retiring_unpolled_single_and_sequence_performs_no_input() {
    // Arrange
    let (_, _, services, client, _) = setup(2, Arc::new(RecordingDelay::default()));
    let mut first = services.mouse_down.start(down()).unwrap();
    let mut second = services.mouse_click.start(click()).unwrap();
    // Act
    first.retire().await.unwrap();
    second.retire().await.unwrap();
    // Assert
    assert!(client.calls().is_empty());
    assert_eq!(
        first.result().await.unwrap_err().kind(),
        FailureKind::ProtocolViolation
    );
}
#[tokio::test]
async fn retiring_interpolation_stops_future_steps_without_releasing_held_input() {
    // Arrange
    let (_, _, services, client, _) = setup(2, Arc::new(RecordingDelay::default()));
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_pointer_gates([Some(gate.clone())]);
    let mut movement = services
        .mouse_move
        .start(MoveParams {
            x: number(100.0),
            y: number(80.0),
            steps: MouseMoveSteps::try_from(4.0).unwrap(),
        })
        .unwrap();
    let mut result = Box::pin(movement.result());
    assert!(
        result
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    drop(result);
    // Act
    gate.notify_one();
    movement.retire().await.unwrap();
    // Assert
    assert_eq!(client.calls().len(), 1);
    let mut press = services.mouse_down.start(down()).unwrap();
    press.result().await.unwrap();
    press.retire().await.unwrap();
    assert_eq!(client.calls()[1].1[0]["x"], 100.0);
}
#[tokio::test]
async fn cancelling_delayed_click_preserves_pressed_button_for_next_raw_move() {
    // Arrange
    let (_, _, services, client, _) = setup(2, Arc::new(PendingDelay));
    let mut params = click();
    params.delay = Some(Duration::from_secs(1));
    let mut operation = services.mouse_click.start(params).unwrap();
    let mut result = Box::pin(operation.result());
    assert!(
        result
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    drop(result);
    // Act
    operation.retire().await.unwrap();
    let mut movement = services
        .mouse_move
        .start(MoveParams {
            x: number(200.0),
            y: number(210.0),
            steps: MouseMoveSteps::ONE,
        })
        .unwrap();
    movement.result().await.unwrap();
    movement.retire().await.unwrap();
    // Assert
    assert_eq!(client.calls().len(), 3);
    assert_eq!(client.calls()[2].1[0]["buttons"], 1);
}
#[test]
fn exclusive_binding_rejects_before_native_work() {
    // Arrange
    let (factory, _, _, client, _) = setup(1, Arc::new(RecordingDelay::default()));
    // Act
    let result = factory.bind(client.clone(), CdpInputCoordinator::new());
    // Assert
    assert!(matches!(result,Err(error) if error.kind()==FailureKind::UnsupportedCapability));
    assert!(client.calls().is_empty());
}
#[test]
fn overflow_is_rejected_before_constructing_profile() {
    // Arrange
    let count = NonZeroUsize::new(usize::MAX).unwrap();
    // Act
    let result =
        CdpMouseProductionProfile::new(ResourceScope::extension(14, 32), usize::MAX, count);
    // Assert
    assert!(matches!(
        result,
        Err(CdpMouseProfileError::ByteBudgetOverflow)
    ));
}

#[test]
fn ledger_rejection_rolls_back_operation_count_before_retry() {
    // Arrange
    let (factory, resources, services, _, _) = setup(1, Arc::new(RecordingDelay::default()));
    let held = resources
        .claim(factory.profile().requirements())
        .unwrap()
        .reserve(
            factory.profile().scope(),
            ResourceClass::BackendStateBytes,
            1,
        )
        .unwrap();
    // Act
    let result = services.mouse_down.start(down());
    // Assert
    assert!(matches!(result, Err(error) if error.kind() == FailureKind::ResourceExhausted));
    assert_eq!(factory.active_operations(), 0);
    drop(held);
    let operation = services.mouse_down.start(down()).unwrap();
    assert_eq!(factory.active_operations(), 1);
    drop(operation);
    assert_eq!(factory.active_operations(), 0);
}

#[test]
fn binding_rejects_profile_smaller_than_concrete_owner_storage() {
    // Arrange
    let profile = CdpMouseProductionProfile::new(
        ResourceScope::extension(14, 32),
        1,
        NonZeroUsize::new(1).unwrap(),
    )
    .unwrap();
    let requirements = profile.requirements();
    let resources = ProductionResourceLedger::new(
        ResourceLimits::deny_all(1, 1).with_class(ResourceClass::BackendStateBytes, 1, 1),
        requirements,
    )
    .unwrap();
    let factory = CdpMouseServiceFactory::new_with_production_resources(
        profile,
        &resources,
        Arc::new(RecordingDelay::default()),
    )
    .unwrap();
    let client = Arc::new(FakeCdpClient::from_html(""));
    // Act
    let result = factory.bind(
        client.clone(),
        CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved),
    );
    // Assert
    assert!(matches!(result,Err(error) if error.kind()==FailureKind::ResourceExhausted));
    assert_eq!(factory.active_operations(), 0);
    assert!(client.calls().is_empty());
}

#[tokio::test]
async fn escaped_click_owner_retains_admitted_page_input_after_binding_drop() {
    // Arrange
    use pwright_backend_cdp_input::{CdpInputProductionProfile, CdpInputServiceFactory};
    let (factory, _, _, client, _) = setup(1, Arc::new(RecordingDelay::default()));
    let profile = CdpInputProductionProfile::new(
        ResourceScope::extension(14, 33),
        CdpInputPolicy::Interleaved,
        1,
    );
    let requirements = profile.requirements();
    let bytes = requirements.class(ResourceClass::BackendStateBytes);
    let resources = ProductionResourceLedger::new(
        ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
            ResourceClass::BackendStateBytes,
            bytes.global,
            bytes.per_scope,
        ),
        requirements,
    )
    .unwrap();
    let input_factory =
        CdpInputServiceFactory::new_with_production_resources(profile, &resources).unwrap();
    let input = input_factory.create().unwrap();
    let binding = factory.bind(client, input.clone()).unwrap();
    let mut operation = binding.mouse_click.start(click()).unwrap();
    // Act
    drop(binding);
    drop(input);
    operation.result().await.unwrap();
    operation.retire().await.unwrap();
    // Assert
    assert_eq!(input_factory.active_states(), 1);
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
    drop(operation);
    assert_eq!(input_factory.active_states(), 0);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}
#[test]
fn oversized_native_diagnostic_is_bounded_and_resource_classified() {
    // Arrange
    let error = CdpError::Other("中".repeat(MOUSE_ERROR_MAX_BYTES));
    // Act
    let error = native_error(error);
    // Assert
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    assert!(error.message().len() < MOUSE_ERROR_MAX_BYTES);
}
