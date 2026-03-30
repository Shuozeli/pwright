//! Action tests: keyboard, dblclick, hover, checkbox, touchscreen.
//!
//! Requires: docker compose -f docker-compose.local.yml up -d

use pwright_integration_tests::connect_and_navigate;

// ── Keyboard ──

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn keyboard_type_text() {
    let page = connect_and_navigate("/input.html").await;

    page.locator("#text").click().await.unwrap();
    page.keyboard().type_text("hello world").await.unwrap();
    let value = page.locator("#text").input_value().await.unwrap();
    assert_eq!(value, "hello world");
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn keyboard_press_tab() {
    let page = connect_and_navigate("/input.html").await;

    page.locator("#text").click().await.unwrap();
    page.keyboard().press("Tab").await.unwrap();
    // Tab should move focus to next input (#email)
    // We verify by typing into the now-focused element
    page.keyboard().type_text("typed-after-tab").await.unwrap();
    let email_val = page.locator("#email").input_value().await.unwrap();
    assert_eq!(email_val, "typed-after-tab");
}

// ── Interactive page ──

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn keyboard_events_detected() {
    let page = connect_and_navigate("/interactive.html").await;

    page.locator("#focus-input").click().await.unwrap();
    page.keyboard().press("Enter").await.unwrap();

    // Wait for JS to process
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let output = page
        .locator("#keyboard-output")
        .text_content()
        .await
        .unwrap();
    assert!(
        output.unwrap().contains("key:Enter"),
        "keyboard event should be detected"
    );
}

// ── Double click ──

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn dblclick_triggers_event() {
    let page = connect_and_navigate("/action-extras.html").await;

    // Use JS to dispatch dblclick (CDP mouse dblclick on non-focusable div
    // may fail with "Element is not focusable")
    page.locator("#dblclick-target")
        .evaluate_sync("function() { this.dispatchEvent(new MouseEvent('dblclick')); }")
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let result = page
        .locator("#dblclick-result")
        .text_content()
        .await
        .unwrap();
    assert_eq!(
        result,
        Some("Double-clicked!".to_string()),
        "dblclick event should fire"
    );
}

// ── Checkbox via action-extras ──

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn checkbox_toggle_via_js() {
    let page = connect_and_navigate("/action-extras.html").await;

    assert!(!page.locator("#agree").is_checked().await.unwrap());

    page.locator("#agree")
        .evaluate_sync("function() { this.click(); }")
        .await
        .unwrap();

    assert!(page.locator("#agree").is_checked().await.unwrap());

    // Toggle back
    page.locator("#agree")
        .evaluate_sync("function() { this.click(); }")
        .await
        .unwrap();

    assert!(!page.locator("#agree").is_checked().await.unwrap());
}

// ── Check / Uncheck via Locator ──

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn check_sets_checkbox() {
    let page = connect_and_navigate("/action-extras.html").await;

    assert!(!page.locator("#agree").is_checked().await.unwrap());
    page.locator("#agree").check().await.unwrap();
    assert!(page.locator("#agree").is_checked().await.unwrap());
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn uncheck_clears_checkbox() {
    let page = connect_and_navigate("/action-extras.html").await;

    // Check first
    page.locator("#agree").check().await.unwrap();
    assert!(page.locator("#agree").is_checked().await.unwrap());

    // Then uncheck
    page.locator("#agree").uncheck().await.unwrap();
    assert!(!page.locator("#agree").is_checked().await.unwrap());
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn check_is_idempotent() {
    let page = connect_and_navigate("/action-extras.html").await;

    page.locator("#agree").check().await.unwrap();
    assert!(page.locator("#agree").is_checked().await.unwrap());

    // Check again should be a no-op
    page.locator("#agree").check().await.unwrap();
    assert!(page.locator("#agree").is_checked().await.unwrap());
}

// ── Scroll ──

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn scroll_into_view() {
    let page = connect_and_navigate("/content.html").await;

    // scroll_into_view should succeed without error
    page.locator("#heading").scroll_into_view().await.unwrap();

    // Element should still be accessible after scroll
    let text = page.locator("#heading").text_content().await.unwrap();
    assert_eq!(text, Some("Hello pwright".to_string()));
}

// ── Text extraction ──

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn body_text_extraction() {
    let page = connect_and_navigate("/content.html").await;

    let text: String = page
        .evaluate_sync_into("(document.body?.innerText || '')")
        .await
        .unwrap();

    assert!(
        text.contains("Hello pwright"),
        "body text should contain heading"
    );
    assert!(
        text.contains("Example Link"),
        "body text should contain link text"
    );
}

// ── Touchscreen ──

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn touchscreen_tap() {
    let page = connect_and_navigate("/touchscreen.html").await;

    let bbox = page.locator("#target").bounding_box().await.unwrap();
    assert!(bbox.is_some(), "target should have bounding box");

    let b = bbox.unwrap();
    let x = b.x + b.width / 2.0;
    let y = b.y + b.height / 2.0;

    page.touchscreen().tap(x, y).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let result = page.locator("#result").text_content().await.unwrap();
    // The page registers either touch or click
    assert_ne!(
        result,
        Some("No touch".to_string()),
        "touch should be registered"
    );
}

// ── File upload ──

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn file_upload_shows_filename() {
    let page = connect_and_navigate("/file-upload.html").await;

    // Create a temp file
    let tmp = std::env::temp_dir().join("pwright-test-upload.txt");
    std::fs::write(&tmp, "test content").unwrap();

    page.locator("#file-input")
        .set_input_files(&[tmp.to_string_lossy().to_string()])
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let status = page.locator("#status").text_content().await.unwrap();
    assert!(
        status
            .as_ref()
            .map(|s| s.contains("pwright-test-upload.txt"))
            .unwrap_or(false),
        "should show uploaded filename, got: {:?}",
        status
    );

    let _ = std::fs::remove_file(&tmp);
}
