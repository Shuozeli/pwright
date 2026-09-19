use super::*;
use pwright_backend_cdp_input::{
    DirectMouseDispatch, MouseDispatchKind, MouseDispatchOperation, MouseDispatchRequest,
};

#[derive(Default)]
struct Recorder(Mutex<Vec<MouseDispatchKind>>);
impl CdpMouseDispatcher for Recorder {
    fn start(
        &self,
        client: Arc<dyn CdpClient>,
        request: MouseDispatchRequest,
    ) -> Box<dyn MouseDispatchOperation> {
        self.0.lock().unwrap().push(request.kind);
        Box::new(DirectMouseDispatch::new(client, request.event))
    }
}

#[tokio::test]
async fn page_binding_selects_one_dispatcher_for_all_pointer_actions_but_not_wheel() {
    // Arrange
    let (factory, _, _, client, input) = setup(1, Arc::new(RecordingDelay::default()));
    let recorder = Arc::new(Recorder::default());
    let services = factory
        .bind_with_dispatcher(client.clone(), input, recorder.clone())
        .unwrap();
    macro_rules! exercise {
        ($service:ident, $params:expr) => {{
            let mut owner = services.$service.start($params).unwrap();
            owner.result().await.unwrap();
            owner.retire().await.unwrap();
            drop(owner);
        }};
    }
    // Act
    exercise!(
        mouse_move,
        MoveParams {
            x: number(20.0),
            y: number(30.0),
            steps: MouseMoveSteps::ONE
        }
    );
    exercise!(mouse_down, down());
    exercise!(
        mouse_up,
        UpParams {
            button: MouseButton::Left,
            click_count: 1
        }
    );
    exercise!(mouse_click, click());
    exercise!(
        mouse_dblclick,
        DblclickParams {
            x: number(20.0),
            y: number(30.0),
            button: MouseButton::Left,
            delay: None
        }
    );
    exercise!(
        mouse_wheel,
        WheelParams {
            delta_x: number(1.0),
            delta_y: number(2.0)
        }
    );
    // Assert
    assert_eq!(
        *recorder.0.lock().unwrap(),
        [
            MouseDispatchKind::Move { for_click: false },
            MouseDispatchKind::Down,
            MouseDispatchKind::Up,
            MouseDispatchKind::Move { for_click: true },
            MouseDispatchKind::Down,
            MouseDispatchKind::Up,
            MouseDispatchKind::Move { for_click: true },
            MouseDispatchKind::Down,
            MouseDispatchKind::Up,
            MouseDispatchKind::Down,
            MouseDispatchKind::Up,
        ]
    );
    assert_eq!(client.calls().len(), 12);
    assert_eq!(factory.active_operations(), 0);
}

#[tokio::test]
async fn rejected_admission_and_unpolled_cancellation_do_not_enter_selected_dispatcher() {
    // Arrange
    let (factory, resources, _, client, input) = setup(1, Arc::new(RecordingDelay::default()));
    let recorder = Arc::new(Recorder::default());
    let services = factory
        .bind_with_dispatcher(client.clone(), input, recorder.clone())
        .unwrap();
    let mut owner = services.mouse_down.start(down()).unwrap();
    // Act
    let rejected = services.mouse_down.start(down()).err().unwrap();
    owner.retire().await.unwrap();
    // Assert
    assert_eq!(rejected.kind(), FailureKind::ResourceExhausted);
    assert!(recorder.0.lock().unwrap().is_empty());
    assert!(client.calls().is_empty());
    assert_eq!(factory.active_operations(), 1);
    drop(owner);
    assert_eq!(factory.active_operations(), 0);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn separately_bound_page_does_not_inherit_dispatcher_selection() {
    // Arrange
    let (factory, _, plain, client, input) = setup(2, Arc::new(RecordingDelay::default()));
    let recorder = Arc::new(Recorder::default());
    let other_input = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
    assert!(!other_input.shares_state_with(&input));
    let selected = factory
        .bind_with_dispatcher(client.clone(), other_input, recorder.clone())
        .unwrap();
    let mut delegated = selected.mouse_down.start(down()).unwrap();
    // Act
    delegated.result().await.unwrap();
    delegated.retire().await.unwrap();
    let mut direct = plain.mouse_down.start(down()).unwrap();
    direct.result().await.unwrap();
    direct.retire().await.unwrap();
    // Assert
    assert_eq!(*recorder.0.lock().unwrap(), [MouseDispatchKind::Down]);
    assert_eq!(client.calls().len(), 2);
}

struct SuppressButtons;
struct Suppressed;
impl MouseDispatchOperation for Suppressed {
    fn poll_result(&mut self, _: &mut Context<'_>) -> std::task::Poll<Result<(), CdpError>> {
        std::task::Poll::Ready(Ok(()))
    }
    fn poll_retire(&mut self, _: &mut Context<'_>) -> std::task::Poll<Result<(), CdpError>> {
        std::task::Poll::Ready(Ok(()))
    }
    fn is_retired(&self) -> bool {
        true
    }
}
impl CdpMouseDispatcher for SuppressButtons {
    fn start(
        &self,
        client: Arc<dyn CdpClient>,
        request: MouseDispatchRequest,
    ) -> Box<dyn MouseDispatchOperation> {
        match request.kind {
            MouseDispatchKind::Move { .. } => {
                Box::new(DirectMouseDispatch::new(client, request.event))
            }
            MouseDispatchKind::Down | MouseDispatchKind::Up => Box::new(Suppressed),
        }
    }
}
#[tokio::test]
async fn provider_suppression_changes_physical_dispatch_but_not_logical_page_buttons() {
    // Arrange
    let (factory, _, _, client, input) = setup(1, Arc::new(RecordingDelay::default()));
    let services = factory
        .bind_with_dispatcher(client.clone(), input, Arc::new(SuppressButtons))
        .unwrap();
    let mut down = services.mouse_down.start(down()).unwrap();
    // Act
    down.result().await.unwrap();
    down.retire().await.unwrap();
    drop(down);
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
    let mut up = services
        .mouse_up
        .start(UpParams {
            button: MouseButton::Left,
            click_count: 1,
        })
        .unwrap();
    up.result().await.unwrap();
    up.retire().await.unwrap();
    drop(up);
    let mut movement = services
        .mouse_move
        .start(MoveParams {
            x: number(40.0),
            y: number(50.0),
            steps: MouseMoveSteps::ONE,
        })
        .unwrap();
    movement.result().await.unwrap();
    movement.retire().await.unwrap();
    // Assert
    let calls = client.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].1[0]["type"], "mouseMoved");
    assert_eq!(calls[0].1[0]["buttons"], 1);
    assert_eq!(calls[1].1[0]["buttons"], 0);
}
