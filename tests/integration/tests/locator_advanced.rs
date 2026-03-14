//! Advanced locator tests: getBy* selectors, composition, filtering.
//!
//! Requires: docker compose -f docker-compose.local.yml up -d

use pwright_integration_tests::connect_and_navigate;

// ── get_by_text ──

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn get_by_text_finds_exact_match() {
    let page = connect_and_navigate("/locator-text.html").await;

    let text = page
        .get_by_text("Welcome", true)
        .text_content()
        .await
        .unwrap();
    assert_eq!(text, Some("Welcome".to_string()));
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn get_by_text_finds_substring() {
    let page = connect_and_navigate("/locator-text.html").await;

    let text = page
        .get_by_text("Hello", false)
        .text_content()
        .await
        .unwrap();
    assert!(
        text.unwrap().contains("Hello"),
        "should find element containing 'Hello'"
    );
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn get_by_text_finds_button() {
    let page = connect_and_navigate("/locator-text.html").await;

    let btn = page.get_by_text("Click Me", false);
    let visible = btn.is_visible().await.unwrap();
    assert!(visible);
}

// ── get_by_label ──

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn get_by_label_finds_input_by_for_attribute() {
    let page = connect_and_navigate("/locator-label.html").await;

    let input = page.get_by_label("Email");
    let visible = input.is_visible().await.unwrap();
    assert!(visible, "should find email input by label");
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn get_by_label_finds_input_by_aria_label() {
    let page = connect_and_navigate("/locator-label.html").await;

    let input = page.get_by_label("Search");
    let visible = input.is_visible().await.unwrap();
    assert!(visible, "should find search input by aria-label");
}

// ── get_by_role ──

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn get_by_role_finds_button() {
    let page = connect_and_navigate("/locator-role.html").await;

    let btn = page.get_by_role("button", None);
    let visible = btn.is_visible().await.unwrap();
    assert!(visible);
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn get_by_role_finds_link() {
    let page = connect_and_navigate("/locator-role.html").await;

    let link = page.get_by_role("link", None);
    let visible = link.is_visible().await.unwrap();
    assert!(visible);
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn get_by_role_finds_heading() {
    let page = connect_and_navigate("/locator-role.html").await;

    let heading = page.get_by_role("heading", None);
    let text = heading.text_content().await.unwrap();
    assert!(text.is_some(), "should find heading");
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn get_by_role_finds_checkbox_by_name() {
    let page = connect_and_navigate("/locator-role.html").await;

    let checkbox = page.get_by_role("checkbox", Some("I agree"));
    let visible = checkbox.is_visible().await.unwrap();
    assert!(visible);
}

// ── Locator composition ──

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn locator_and_composition() {
    let page = connect_and_navigate("/locator-compose.html").await;

    let primary_buttons = page.locator("button").and(&page.locator(".primary"));
    let count = primary_buttons.count().await.unwrap();
    assert!(count >= 1, "should find at least one primary button");
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn locator_or_composition() {
    let page = connect_and_navigate("/locator-compose.html").await;

    let buttons_or_links = page.locator("button").or(&page.locator("a"));
    let count = buttons_or_links.count().await.unwrap();
    assert!(count >= 2, "should find buttons or links: got {count}");
}

// ── Filter ──

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn locator_filter_by_text() {
    let page = connect_and_navigate("/locator-filter.html").await;

    let items = page.locator(".item");
    let banana = items.filter_by_text("Banana");
    let text = banana.text_content().await.unwrap();
    assert!(
        text.unwrap().contains("Banana"),
        "filtered locator should find Banana"
    );
}
