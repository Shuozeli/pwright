//! Integration tests for coordinate-based actions (click-at, hover-at, dblclick).
//!
//! These test real CDP `Input.dispatchMouseEvent` at viewport coordinates,
//! which is the mechanism that works on SPAs with empty accessibility trees.
//!
//! Requires: docker compose -f tests/integration/docker-compose.local.yml up -d

use pwright_bridge::FromEvalJson;
use pwright_bridge::playwright::ClickOptions;
use pwright_cdp::MouseButton;
use pwright_integration_tests::connect_and_navigate;

#[derive(serde::Deserialize)]
struct Rect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

/// Helper: get center coordinates of an element via JS getBoundingClientRect.
async fn element_center(page: &pwright_bridge::playwright::Page, selector: &str) -> (f64, f64) {
    let js = format!(
        r##"JSON.stringify((() => {{ var r = document.querySelector('{}').getBoundingClientRect(); return {{ x: r.x, y: r.y, width: r.width, height: r.height }}; }})())"##,
        selector
    );
    let rect: FromEvalJson<Rect> = page.evaluate_sync_into(&js).await.unwrap();
    let r = rect.0;
    (r.x + r.width / 2.0, r.y + r.height / 2.0)
}

/// Click at coordinates triggers a real click event.
#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn click_at_triggers_click_event() {
    let page = connect_and_navigate("/coordinate-actions.html").await;
    let (x, y) = element_center(&page, "#click-target").await;

    page.mouse().click(x, y, None).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let result: String = page
        .evaluate_sync_into("document.getElementById('click-result').textContent")
        .await
        .unwrap();
    assert_eq!(result, "Clicked!");
}

/// Hover at coordinates triggers mouseenter.
#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn hover_at_triggers_mouseenter() {
    let page = connect_and_navigate("/coordinate-actions.html").await;
    let (x, y) = element_center(&page, "#hover-target").await;

    page.mouse().move_to(x, y).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let result: String = page
        .evaluate_sync_into("document.getElementById('hover-result').textContent")
        .await
        .unwrap();
    assert_eq!(result, "Hovered!");
}

/// Double-click at coordinates triggers dblclick event.
#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn dblclick_at_triggers_dblclick_event() {
    let page = connect_and_navigate("/coordinate-actions.html").await;
    let (x, y) = element_center(&page, "#dblclick-target").await;

    page.mouse().dblclick(x, y).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let result: String = page
        .evaluate_sync_into("document.getElementById('dblclick-result').textContent")
        .await
        .unwrap();
    assert_eq!(result, "Double-clicked!");
}

/// Right-click at coordinates triggers contextmenu event.
#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn right_click_at_triggers_contextmenu() {
    let page = connect_and_navigate("/coordinate-actions.html").await;
    let (x, y) = element_center(&page, "#rightclick-target").await;

    page.mouse()
        .click(
            x,
            y,
            Some(ClickOptions {
                button: Some(MouseButton::Right),
                ..Default::default()
            }),
        )
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let result: String = page
        .evaluate_sync_into("document.getElementById('rightclick-result').textContent")
        .await
        .unwrap();
    assert_eq!(result, "Right-clicked!");
}

/// dblclick via interactive.html button.
#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn dblclick_on_interactive_button() {
    let page = connect_and_navigate("/interactive.html").await;
    let (x, y) = element_center(&page, "#dblclick").await;

    page.mouse().dblclick(x, y).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let text: String = page
        .evaluate_sync_into("document.getElementById('dblclick').textContent")
        .await
        .unwrap();
    assert_eq!(text, "Double clicked!");
}

/// Hover on interactive page shows hover text.
#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn hover_on_interactive_div() {
    let page = connect_and_navigate("/interactive.html").await;
    let (x, y) = element_center(&page, "#hover").await;

    page.mouse().move_to(x, y).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let text: String = page
        .evaluate_sync_into("document.getElementById('hover').textContent")
        .await
        .unwrap();
    assert_eq!(text, "Hovered!");
}
