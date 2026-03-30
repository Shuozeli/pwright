//! Integration tests using FakeCdpClient with real Page/Locator logic.
//!
//! Unlike playwright_comparison.rs which uses MockCdpClient (canned responses),
//! these tests use FakeCdpClient with an in-memory DOM that CDP operations
//! actually work against.

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_bridge::playwright::Page;
    use pwright_fake::FakeCdpClient;

    fn page_with_html(html: &str) -> (Arc<FakeCdpClient>, Page) {
        let fake = Arc::new(FakeCdpClient::from_html(html));
        let page = Page::new(fake.clone());
        (fake, page)
    }

    // ═══════════════════════════════════════════════════════════
    //  Locator Queries (behavior verification)
    // ═══════════════════════════════════════════════════════════

    #[tokio::test]
    async fn is_checked_returns_true_for_checked_checkbox() {
        let (_fake, page) = page_with_html(
            r#"
            <input type="checkbox" id="agree" checked />
        "#,
        );

        assert!(page.locator("#agree").is_checked().await.unwrap());
    }

    #[tokio::test]
    async fn is_checked_returns_false_for_unchecked_checkbox() {
        let (_fake, page) = page_with_html(
            r#"
            <input type="checkbox" id="terms" />
        "#,
        );

        assert!(!page.locator("#terms").is_checked().await.unwrap());
    }

    #[tokio::test]
    async fn is_disabled_returns_true_for_disabled_button() {
        let (_fake, page) = page_with_html(
            r#"
            <button class="submit" disabled>Submit</button>
        "#,
        );

        assert!(page.locator("button.submit").is_disabled().await.unwrap());
    }

    #[tokio::test]
    async fn is_disabled_returns_false_for_enabled_button() {
        let (_fake, page) = page_with_html(
            r#"
            <button class="submit">Submit</button>
        "#,
        );

        assert!(!page.locator("button.submit").is_disabled().await.unwrap());
    }

    #[tokio::test]
    async fn is_enabled_inverse_of_disabled() {
        let (_fake, page) =
            page_with_html(r##"<button disabled>Disabled</button><a href="#">Enabled</a>"##);

        let btn_disabled = page.locator("button").is_enabled().await.unwrap();
        assert!(!btn_disabled);
        let link_enabled = page.locator("a").is_enabled().await.unwrap();
        assert!(link_enabled);
    }

    #[tokio::test]
    async fn is_visible_returns_true_for_visible_element() {
        let (_fake, page) = page_with_html(r#"<div id="visible">Hello</div>"#);

        assert!(page.locator("#visible").is_visible().await.unwrap());
    }

    #[tokio::test]
    async fn text_content_returns_element_text() {
        let (_fake, page) = page_with_html(
            r#"
            <h1>Welcome to pwright</h1>
            <p>A lightweight browser automation tool</p>
        "#,
        );

        let text = page.locator("h1").text_content().await.unwrap();
        assert_eq!(text, Some("Welcome to pwright".to_string()));

        let para = page.locator("p").text_content().await.unwrap();
        assert_eq!(
            para,
            Some("A lightweight browser automation tool".to_string())
        );
    }

    #[tokio::test]
    async fn text_content_nested_elements() {
        let (_fake, page) =
            page_with_html(r#"<div id="container"><span>Hello </span><span>World</span></div>"#);

        let text = page.locator("#container").text_content().await.unwrap();
        assert_eq!(text, Some("Hello World".to_string()));
    }

    #[tokio::test]
    async fn get_attribute_returns_value() {
        let (_fake, page) = page_with_html(
            r#"
            <a href="https://example.com" class="link" target="_blank">Example</a>
        "#,
        );

        let href = page.locator("a").get_attribute("href").await.unwrap();
        assert_eq!(href, Some("https://example.com".to_string()));

        let target = page.locator("a").get_attribute("target").await.unwrap();
        assert_eq!(target, Some("_blank".to_string()));

        let missing = page.locator("a").get_attribute("data-nope").await.unwrap();
        assert_eq!(missing, None);
    }

    #[tokio::test]
    async fn count_returns_number_of_matches() {
        let (_fake, page) = page_with_html(
            r#"
            <ul>
                <li>Item 1</li>
                <li>Item 2</li>
                <li>Item 3</li>
            </ul>
        "#,
        );

        assert_eq!(page.locator("li").count().await.unwrap(), 3);
        assert_eq!(page.locator("ul").count().await.unwrap(), 1);
        assert_eq!(page.locator(".nonexistent").count().await.unwrap(), 0);
    }

    // ═══════════════════════════════════════════════════════════
    //  Locator Composition (first/last/nth)
    // ═══════════════════════════════════════════════════════════

    #[tokio::test]
    async fn first_picks_first_element() {
        let (_fake, page) = page_with_html(
            r#"
            <ul>
                <li>Alpha</li>
                <li>Beta</li>
                <li>Gamma</li>
            </ul>
        "#,
        );

        let text = page.locator("li").first().text_content().await.unwrap();
        assert_eq!(text, Some("Alpha".to_string()));
    }

    #[tokio::test]
    async fn last_picks_last_element() {
        let (_fake, page) = page_with_html(
            r#"
            <ul>
                <li>Alpha</li>
                <li>Beta</li>
                <li>Gamma</li>
            </ul>
        "#,
        );

        let text = page.locator("li").last().text_content().await.unwrap();
        assert_eq!(text, Some("Gamma".to_string()));
    }

    #[tokio::test]
    async fn nth_picks_correct_element() {
        let (_fake, page) = page_with_html(
            r#"
            <ul>
                <li>Alpha</li>
                <li>Beta</li>
                <li>Gamma</li>
            </ul>
        "#,
        );

        let text = page.locator("li").nth(1).text_content().await.unwrap();
        assert_eq!(text, Some("Beta".to_string()));
    }

    // ═══════════════════════════════════════════════════════════
    //  Page convenience methods
    // ═══════════════════════════════════════════════════════════

    #[tokio::test]
    async fn page_text_content_with_selector() {
        let (_fake, page) = page_with_html(
            r#"
            <h1>Title</h1>
            <p>Description</p>
        "#,
        );

        let text = page.text_content("h1").await.unwrap();
        assert_eq!(text, Some("Title".to_string()));
    }

    #[tokio::test]
    async fn page_is_visible_with_selector() {
        let (_fake, page) = page_with_html(r#"<button>Click me</button>"#);
        assert!(page.is_visible("button").await.unwrap());
    }

    #[tokio::test]
    async fn page_is_checked_with_selector() {
        let (_fake, page) = page_with_html(
            r#"
            <input type="checkbox" id="a" checked />
            <input type="checkbox" id="b" />
        "#,
        );

        assert!(page.is_checked("#a").await.unwrap());
        assert!(!page.is_checked("#b").await.unwrap());
    }

    #[tokio::test]
    async fn page_get_attribute_with_selector() {
        let (_fake, page) = page_with_html(
            r#"
            <img alt="Logo" src="logo.png" />
        "#,
        );

        let alt = page.get_attribute("img", "alt").await.unwrap();
        assert_eq!(alt, Some("Logo".to_string()));
    }

    // ═══════════════════════════════════════════════════════════
    //  Element not found
    // ═══════════════════════════════════════════════════════════

    #[tokio::test]
    async fn click_nonexistent_returns_element_not_found() {
        let (_fake, page) = page_with_html(r#"<div>empty</div>"#);

        let result = page.locator(".nonexistent").click().await;
        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(err.contains(".nonexistent"), "Error: {err}");
    }

    // ═══════════════════════════════════════════════════════════
    //  Complex page (TodoMVC-like)
    // ═══════════════════════════════════════════════════════════

    #[tokio::test]
    async fn todomvc_like_page() {
        let (_fake, page) = page_with_html(
            r#"
            <section class="todoapp">
                <header>
                    <h1>todos</h1>
                    <input class="new-todo" placeholder="What needs to be done?" />
                </header>
                <ul class="todo-list">
                    <li class="todo">
                        <input class="toggle" type="checkbox" />
                        <label>Buy groceries</label>
                    </li>
                    <li class="todo completed">
                        <input class="toggle" type="checkbox" checked />
                        <label>Walk the dog</label>
                    </li>
                    <li class="todo">
                        <input class="toggle" type="checkbox" />
                        <label>Write tests</label>
                    </li>
                </ul>
                <footer class="footer">
                    <span class="todo-count">2 items left</span>
                </footer>
            </section>
        "#,
        );

        // Count todos
        assert_eq!(page.locator("li.todo").count().await.unwrap(), 3);

        // Check completed status
        let toggles = page.locator(".toggle");
        assert!(!toggles.first().is_checked().await.unwrap());
        assert!(toggles.nth(1).is_checked().await.unwrap());
        assert!(!toggles.last().is_checked().await.unwrap());

        // Get todo text
        let first_label = page
            .locator("li.todo label")
            .first()
            .text_content()
            .await
            .unwrap();
        assert_eq!(first_label, Some("Buy groceries".to_string()));

        let last_label = page
            .locator("li.todo label")
            .last()
            .text_content()
            .await
            .unwrap();
        assert_eq!(last_label, Some("Write tests".to_string()));

        // Footer text
        let count_text = page.locator(".todo-count").text_content().await.unwrap();
        assert_eq!(count_text, Some("2 items left".to_string()));

        // Verify input placeholder
        let placeholder = page
            .locator(".new-todo")
            .get_attribute("placeholder")
            .await
            .unwrap();
        assert_eq!(placeholder, Some("What needs to be done?".to_string()));

        // Completed class selector
        assert_eq!(page.locator("li.completed").count().await.unwrap(), 1);
    }

    // ═══════════════════════════════════════════════════════════
    //  Wait methods
    // ═══════════════════════════════════════════════════════════

    #[tokio::test]
    async fn wait_for_text_finds_existing_text() {
        let (fake, page) = page_with_html(r#"<body><p>Hello World</p></body>"#);
        // Simulate JS `document.body.innerText.includes("Hello")` returning true
        fake.set_evaluate_response(serde_json::json!({
            "result": {"value": true}
        }));

        page.wait_for_text("Hello", 1000).await.unwrap();
    }

    #[tokio::test]
    async fn wait_for_text_times_out_when_text_absent() {
        let (_fake, page) = page_with_html(r#"<body><p>Nothing here</p></body>"#);
        // Default evaluate_response is {"result": {"value": ""}}, which is not true

        let result = page.wait_for_text("Missing text", 250).await;
        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(
            err.contains("Timeout"),
            "Expected timeout error, got: {err}"
        );
    }

    #[tokio::test]
    async fn wait_for_text_in_selector_succeeds() {
        let (fake, page) = page_with_html(r#"<div class="response">Operation complete</div>"#);
        // Simulate the scoped querySelector + includes check returning true
        fake.set_evaluate_response(serde_json::json!({
            "result": {"value": true}
        }));

        page.wait_for_text_in(".response", "complete", 1000)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn wait_for_text_in_times_out() {
        let (_fake, page) = page_with_html(r#"<div class="response">Operation complete</div>"#);
        // Default evaluate_response does not return true

        let result = page.wait_for_text_in(".response", "missing", 250).await;
        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(
            err.contains("Timeout"),
            "Expected timeout error, got: {err}"
        );
    }

    #[tokio::test]
    async fn wait_until_true_expression() {
        let (fake, page) = page_with_html(r#"<div>content</div>"#);
        fake.set_evaluate_response(serde_json::json!({
            "result": {"value": true}
        }));

        page.wait_until("document.title !== ''", 1000)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn wait_until_times_out() {
        let (fake, page) = page_with_html(r#"<div>content</div>"#);
        fake.set_evaluate_response(serde_json::json!({
            "result": {"value": false}
        }));

        let result = page.wait_until("false", 250).await;
        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(
            err.contains("Timeout"),
            "Expected timeout error, got: {err}"
        );
    }

    #[tokio::test]
    async fn wait_for_selector_existing() {
        let (_fake, page) = page_with_html(r#"<button id="submit">Go</button>"#);

        // The fake DOM has the button, so wait_for_selector should resolve immediately
        page.wait_for_selector("#submit", 1000).await.unwrap();
    }

    #[tokio::test]
    async fn wait_for_selector_times_out() {
        let (_fake, page) = page_with_html(r#"<div>empty</div>"#);

        let result = page.wait_for_selector(".nonexistent", 300).await;
        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(
            err.contains("Timeout") || err.contains("timeout"),
            "Expected timeout error, got: {err}"
        );
    }

    #[tokio::test]
    async fn wait_for_timeout_completes() {
        let (_fake, page) = page_with_html(r#"<div>content</div>"#);

        // Just verify it completes without error
        page.wait_for_timeout(50).await.unwrap();
    }
}
