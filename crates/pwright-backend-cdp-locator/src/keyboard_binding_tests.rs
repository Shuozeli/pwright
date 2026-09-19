use super::*;
use pwright_backend_cdp_input::{CdpKeyDownInterceptor, KeyDownInterceptionFuture};
use pwright_cap_locator::{PressKey, PressOptions, PressSequentiallyParams};
use pwright_fake::FakeCdpClient;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Escape {
    page: Arc<dyn CdpClient>,
    calls: AtomicUsize,
}

struct FocusHandoff {
    frame: Arc<FakeCdpClient>,
    page: Arc<FakeCdpClient>,
    calls: AtomicUsize,
    reject: bool,
}
impl LocatorInputHandoff for FocusHandoff {
    fn handoff(&self) -> Result<(), OperationError<LocatorErrorDetails>> {
        assert_eq!(
            self.frame.method_names(),
            ["Runtime.evaluate", "Runtime.evaluate"]
        );
        assert!(
            self.page.calls().is_empty(),
            "input must not start before handoff"
        );
        self.calls.fetch_add(1, Ordering::Relaxed);
        if self.reject {
            Err(OperationError::new(
                FailureKind::TargetClosed,
                "Frame closed before handoff",
                None,
            ))
        } else {
            Ok(())
        }
    }
}

#[tokio::test]
async fn press_hands_off_after_focus_before_page_input() {
    // Arrange
    let (frame, page, service, hook) = prepared();
    let handoff = Arc::new(FocusHandoff {
        frame,
        page: page.clone(),
        calls: AtomicUsize::new(0),
        reject: false,
    });
    let input = service.input.clone();
    let service = service
        .with_keyboard_binding(page.clone(), &input, hook)
        .unwrap()
        .with_keyboard_handoff(handoff.clone())
        .unwrap();
    // Act
    service
        .call(PressParams::new(
            Selector::css("input").unwrap(),
            PressKey::new("Escape").unwrap(),
            PressOptions::new().with_no_wait_after(true),
        ))
        .await
        .unwrap();
    // Assert
    assert_eq!(handoff.calls.load(Ordering::Relaxed), 1);
    assert_eq!(page.method_names(), ["Input.dispatchKeyEvent"]);
}

#[tokio::test]
async fn rejected_handoff_prevents_all_page_key_and_text_input() {
    for sequential in [false, true] {
        // Arrange
        let (frame, page, service, hook) = prepared();
        let handoff = Arc::new(FocusHandoff {
            frame,
            page: page.clone(),
            calls: AtomicUsize::new(0),
            reject: true,
        });
        let input = service.input.clone();
        let service = service
            .with_keyboard_binding(page.clone(), &input, hook.clone())
            .unwrap()
            .with_keyboard_handoff(handoff.clone())
            .unwrap();
        // Act
        let result = if sequential {
            service
                .call(PressSequentiallyParams::new(
                    Selector::css("input").unwrap(),
                    "a中",
                    None,
                ))
                .await
        } else {
            service
                .call(PressParams::new(
                    Selector::css("input").unwrap(),
                    PressKey::new("Escape").unwrap(),
                    PressOptions::new().with_no_wait_after(true),
                ))
                .await
        };
        // Assert
        assert_eq!(result.unwrap_err().kind(), FailureKind::TargetClosed);
        assert_eq!(handoff.calls.load(Ordering::Relaxed), 1);
        assert_eq!(hook.calls.load(Ordering::Relaxed), 0);
        assert!(page.calls().is_empty());
    }
}
impl CdpKeyDownInterceptor for Escape {
    fn intercept(
        self: Arc<Self>,
        client: Arc<dyn CdpClient>,
        code: &'static str,
    ) -> Option<KeyDownInterceptionFuture> {
        assert!(Arc::ptr_eq(&client, &self.page));
        self.calls.fetch_add(1, Ordering::Relaxed);
        (code == "Escape").then(|| Box::pin(async { Ok(true) }) as KeyDownInterceptionFuture)
    }
}
fn prepared() -> (
    Arc<FakeCdpClient>,
    Arc<FakeCdpClient>,
    CdpLocatorService,
    Arc<Escape>,
) {
    let frame = Arc::new(FakeCdpClient::from_html("<input>"));
    frame.set_evaluate_responses([
        json!({"result":{"value":true}}),
        json!({"result":{"value":{"status":"done","isMac":false}}}),
    ]);
    let page = Arc::new(FakeCdpClient::from_html(""));
    let mut service = CdpLocatorService::new(frame.clone());
    service.input = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
    let hook = Arc::new(Escape {
        page: page.clone(),
        calls: AtomicUsize::new(0),
    });
    (frame, page, service, hook)
}

#[tokio::test]
async fn press_accepts_an_oopif_navigation_root_and_finishes_on_the_original_page() {
    // Arrange
    let (frame, page, service, hook) = prepared();
    let mut tree = frame.page_get_frame_tree().await.unwrap();
    tree.frame.id = "oopif".into();
    tree.frame.parent_id = Some("parent".into());
    frame.set_frame_tree(tree);
    let input = service.input.clone();
    let service = service
        .with_keyboard_binding(page.clone(), &input, hook)
        .unwrap();
    // Act
    service
        .call(PressParams::new(
            Selector::css("input").unwrap(),
            PressKey::new("Escape").unwrap(),
            PressOptions::new(),
        ))
        .await
        .unwrap();
    // Assert
    assert_eq!(
        frame.method_names(),
        [
            "Page.getFrameTree",
            "Runtime.evaluate",
            "Page.enable",
            "Page.getFrameTree",
            "Runtime.evaluate"
        ]
    );
    assert_eq!(
        page.method_names(),
        ["Input.dispatchKeyEvent", "Page.enable"]
    );
}

#[tokio::test]
async fn navigation_barrier_tracks_the_renderer_root_even_when_it_has_a_parent() {
    // Arrange
    let frame = FakeCdpClient::from_html("");
    let mut tree = frame.page_get_frame_tree().await.unwrap();
    tree.frame.id = "oopif".into();
    tree.frame.parent_id = Some("parent".into());
    frame.set_frame_tree(tree);
    // Act
    let barrier = ClickNavigationBarrier::prepare(&frame).await.unwrap();
    // Assert
    assert_eq!(barrier.main_frame_id, "oopif");
}

#[tokio::test]
async fn press_focuses_frame_but_intercepts_and_releases_key_on_original_page() {
    // Arrange
    let (frame, page, service, hook) = prepared();
    let input = service.input.clone();
    let service = service
        .with_keyboard_binding(page.clone(), &input, hook.clone())
        .unwrap();
    let params = PressParams::new(
        Selector::css("input").unwrap(),
        PressKey::new("Escape").unwrap(),
        PressOptions::new().with_no_wait_after(true),
    );
    // Act
    service.call(params).await.unwrap();
    // Assert
    assert_eq!(
        frame.method_names(),
        ["Runtime.evaluate", "Runtime.evaluate"]
    );
    assert_eq!(page.method_names(), ["Input.dispatchKeyEvent"]);
    assert_eq!(page.calls()[0].1[0]["type"], "keyUp");
    assert_eq!(hook.calls.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn sequential_typing_keeps_frame_focus_and_page_keyboard_and_insert_text() {
    // Arrange
    let (frame, page, service, hook) = prepared();
    let input = service.input.clone();
    let service = service
        .with_keyboard_binding(page.clone(), &input, hook.clone())
        .unwrap();
    // Act
    service
        .call(PressSequentiallyParams::new(
            Selector::css("input").unwrap(),
            "a中",
            None,
        ))
        .await
        .unwrap();
    // Assert
    assert_eq!(
        frame.method_names(),
        ["Runtime.evaluate", "Runtime.evaluate"]
    );
    assert_eq!(
        page.method_names(),
        [
            "Input.dispatchKeyEvent",
            "Input.dispatchKeyEvent",
            "Input.insertText"
        ]
    );
    assert_eq!(hook.calls.load(Ordering::Relaxed), 1);
}

#[test]
fn keyboard_binding_rejects_foreign_page_without_native_effects() {
    // Arrange
    let (frame, page, service, hook) = prepared();
    let foreign = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
    // Act
    let result = service.with_keyboard_binding(page.clone(), &foreign, hook);
    // Assert
    assert!(result.is_err());
    assert!(frame.calls().is_empty());
    assert!(page.calls().is_empty());
}

#[test]
fn keyboard_binding_rejects_replacement_without_native_effects() {
    // Arrange
    let (frame, page, service, hook) = prepared();
    let input = service.input.clone();
    let service = service
        .with_keyboard_binding(page.clone(), &input, hook.clone())
        .unwrap();
    // Act
    let result = service.with_keyboard_binding(page.clone(), &input, hook);
    // Assert
    assert!(result.is_err());
    assert!(frame.calls().is_empty());
    assert!(page.calls().is_empty());
}

#[test]
fn keyboard_binding_rejects_exclusive_policy_without_native_effects() {
    // Arrange
    let (frame, page, mut service, hook) = prepared();
    service.input = CdpInputCoordinator::with_policy(CdpInputPolicy::Exclusive);
    let input = service.input.clone();
    // Act
    let result = service.with_keyboard_binding(page.clone(), &input, hook);
    // Assert
    assert!(result.is_err());
    assert!(frame.calls().is_empty());
    assert!(page.calls().is_empty());
}
