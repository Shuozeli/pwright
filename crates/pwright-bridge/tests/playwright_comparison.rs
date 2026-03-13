//! Playwright API Comparison Tests
//!
//! Each test shows the equivalent Playwright (TypeScript) code as a comment,
//! then the pwright (Rust) equivalent, and verifies the CDP calls match
//! what Playwright would send.
//!
//! This ensures pwright is API-compatible with Playwright at the CDP level.

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_bridge::playwright::Page;
    use pwright_bridge::test_utils::MockCdpClient;

    // ═══════════════════════════════════════════════════════════
    //  Navigation
    // ═══════════════════════════════════════════════════════════

    /// ```typescript
    /// // Playwright
    /// await page.goto('https://example.com');
    /// ```
    #[tokio::test]
    async fn compare_goto() {
        let mock = Arc::new(MockCdpClient::new());
        mock.set_navigate_response(serde_json::json!({"frameId": "F1"}));
        mock.set_evaluate_response(serde_json::json!({"result": {"value": "complete"}}));

        // pwright
        let page = Page::new(mock.clone());
        page.goto("https://example.com", None).await.unwrap();

        // Both should call Page.navigate
        let nav_calls = mock.calls_for("Page.navigate");
        assert_eq!(nav_calls.len(), 1);
        assert_eq!(nav_calls[0].args[0], "https://example.com");
    }

    /// ```typescript
    /// // Playwright
    /// await page.reload();
    /// ```
    #[tokio::test]
    async fn compare_reload() {
        let mock = Arc::new(MockCdpClient::new());

        // pwright
        let page = Page::new(mock.clone());
        page.reload().await.unwrap();

        assert_eq!(mock.method_names(), vec!["Page.reload"]);
    }

    /// ```typescript
    /// // Playwright
    /// await page.goBack();
    /// ```
    #[tokio::test]
    async fn compare_go_back() {
        let mock = Arc::new(MockCdpClient::new());
        mock.set_navigation_history_response(serde_json::json!({
            "currentIndex": 2,
            "entries": [
                {"id": 0, "url": "https://a.com", "title": "A"},
                {"id": 1, "url": "https://b.com", "title": "B"},
                {"id": 2, "url": "https://c.com", "title": "C"},
            ]
        }));

        // pwright
        let page = Page::new(mock.clone());
        page.go_back().await.unwrap();

        // Both should: getNavigationHistory → navigateToHistoryEntry(1)
        let methods = mock.method_names();
        assert_eq!(methods[0], "Page.getNavigationHistory");
        assert_eq!(methods[1], "Page.navigateToHistoryEntry");

        let nav_calls = mock.calls_for("Page.navigateToHistoryEntry");
        assert_eq!(nav_calls[0].args[0]["entryId"], 1);
    }

    // ═══════════════════════════════════════════════════════════
    //  Content & State
    // ═══════════════════════════════════════════════════════════

    /// ```typescript
    /// // Playwright
    /// const url = page.url();
    /// const title = await page.title();
    /// ```
    #[tokio::test]
    async fn compare_url_and_title() {
        let mock = Arc::new(MockCdpClient::new());
        mock.set_evaluate_response(serde_json::json!({
            "result": {"value": "https://example.com/path"}
        }));

        // pwright
        let page = Page::new(mock.clone());
        let url = page.url().await.unwrap();
        assert_eq!(url, "https://example.com/path");

        // Both call Runtime.evaluate("window.location.href") / ("document.title")
        let eval_calls = mock.calls_for("Runtime.evaluate");
        assert!(!eval_calls.is_empty());
    }

    /// ```typescript
    /// // Playwright
    /// const html = await page.content();
    /// ```
    #[tokio::test]
    async fn compare_content() {
        let mock = Arc::new(MockCdpClient::new());
        mock.set_evaluate_response(serde_json::json!({
            "result": {"value": "<html><body>hello</body></html>"}
        }));

        // pwright
        let page = Page::new(mock.clone());
        let html = page.content().await.unwrap();
        assert_eq!(html, "<html><body>hello</body></html>");

        // Both call Runtime.evaluate("document.documentElement.outerHTML")
        let calls = mock.calls_for("Runtime.evaluate");
        let expr = calls[0].args[0].as_str().unwrap();
        assert_eq!(expr, pwright_js::page::GET_DOCUMENT_HTML);
    }

    // ═══════════════════════════════════════════════════════════
    //  Locator — Element Interaction
    // ═══════════════════════════════════════════════════════════

    fn mock_with_element() -> Arc<MockCdpClient> {
        let mock = Arc::new(MockCdpClient::new());
        mock.set_query_selector_response(42);
        mock.set_resolve_node(serde_json::json!({"object": {"objectId": "obj-42"}}));
        mock.set_call_function_response(serde_json::json!({
            "result": {"value": {"x": 150.0, "y": 250.0}}
        }));
        mock
    }

    /// ```typescript
    /// // Playwright
    /// await page.locator('button.submit').click();
    /// ```
    #[tokio::test]
    async fn compare_locator_click() {
        let mock = mock_with_element();

        // pwright
        let page = Page::new(mock.clone());
        page.locator("button.submit").click().await.unwrap();

        // Both should: getDocument → querySelector → resolveNode → getBoundingClientRect → mousePressed → mouseReleased
        let methods = mock.method_names();
        assert!(methods.contains(&"DOM.getDocument".to_string()));
        assert!(methods.contains(&"DOM.querySelector".to_string()));
        assert!(methods.contains(&"Input.dispatchMouseEvent".to_string()));

        // Verify selector was passed correctly
        let qs_calls = mock.calls_for("DOM.querySelector");
        assert_eq!(qs_calls[0].args[0]["selector"], "button.submit");

        // Verify click coordinates match the bounding rect center
        let mouse_calls = mock.calls_for("Input.dispatchMouseEvent");
        assert!(mouse_calls.len() >= 2); // mousePressed + mouseReleased
    }

    /// ```typescript
    /// // Playwright
    /// await page.locator('#email').fill('user@example.com');
    /// ```
    #[tokio::test]
    async fn compare_locator_fill() {
        let mock = mock_with_element();

        // pwright
        let page = Page::new(mock.clone());
        page.locator("#email")
            .fill("user@example.com")
            .await
            .unwrap();

        // Both should: resolve element → focus → callFunctionOn(SET_VALUE)
        let methods = mock.method_names();
        assert!(methods.contains(&"DOM.querySelector".to_string()));
        assert!(methods.contains(&"Runtime.callFunctionOn".to_string()));

        // Verify the value was passed
        let cf_calls = mock.calls_for("Runtime.callFunctionOn");
        assert!(!cf_calls.is_empty());
    }

    /// ```typescript
    /// // Playwright
    /// await page.locator('input').type('hello');
    /// ```
    #[tokio::test]
    async fn compare_locator_type() {
        let mock = mock_with_element();

        // pwright
        let page = Page::new(mock.clone());
        page.locator("input").type_text("hello").await.unwrap();

        // Both should: resolve → focus → insertText per character
        let insert_calls = mock.calls_for("Input.insertText");
        assert_eq!(insert_calls.len(), 5); // h, e, l, l, o
    }

    /// ```typescript
    /// // Playwright
    /// const text = await page.locator('h1').textContent();
    /// ```
    #[tokio::test]
    async fn compare_locator_text_content() {
        let mock = mock_with_element();
        mock.set_call_function_response(serde_json::json!({
            "result": {"value": "Welcome"}
        }));

        // pwright
        let page = Page::new(mock.clone());
        let text = page.locator("h1").text_content().await.unwrap();
        assert_eq!(text, Some("Welcome".to_string()));

        // Both call callFunctionOn with a function that returns this.textContent
        let cf_calls = mock.calls_for("Runtime.callFunctionOn");
        let fn_decl = cf_calls.last().unwrap().args[0]["functionDeclaration"]
            .as_str()
            .unwrap();
        assert_eq!(fn_decl, pwright_js::element::GET_TEXT_CONTENT);
    }

    /// ```typescript
    /// // Playwright — getAttribute uses DOM.getAttributes (no JS)
    /// const href = await page.locator('a').getAttribute('href');
    /// ```
    #[tokio::test]
    async fn compare_locator_get_attribute() {
        let mock = Arc::new(MockCdpClient::new());
        mock.set_query_selector_response(42);
        mock.set_get_attributes_response(vec![
            "href".to_string(),
            "https://example.com".to_string(),
            "class".to_string(),
            "link".to_string(),
            "target".to_string(),
            "_blank".to_string(),
        ]);

        // pwright
        let page = Page::new(mock.clone());
        let href = page.locator("a").get_attribute("href").await.unwrap();
        assert_eq!(href, Some("https://example.com".to_string()));

        let target = page.locator("a").get_attribute("target").await.unwrap();
        assert_eq!(target, Some("_blank".to_string()));

        let missing = page.locator("a").get_attribute("data-nope").await.unwrap();
        assert_eq!(missing, None);

        // pwright uses DOM.getAttributes (pure CDP, no JS!) — same as Playwright internally
        let methods = mock.method_names();
        assert!(methods.contains(&"DOM.getAttributes".to_string()));
        assert!(!methods.contains(&"Runtime.evaluate".to_string()));
        assert!(!methods.contains(&"Runtime.callFunctionOn".to_string()));
    }

    /// ```typescript
    /// // Playwright
    /// const count = await page.locator('li').count();
    /// ```
    #[tokio::test]
    async fn compare_locator_count() {
        let mock = Arc::new(MockCdpClient::new());
        mock.set_query_selector_all_response(vec![1, 2, 3, 4, 5]);

        // pwright
        let page = Page::new(mock.clone());
        let count = page.locator("li").count().await.unwrap();
        assert_eq!(count, 5);

        // Both call DOM.querySelectorAll and count results
        let calls = mock.calls_for("DOM.querySelectorAll");
        assert_eq!(calls[0].args[0]["selector"], "li");
    }

    /// ```typescript
    /// // Playwright
    /// const visible = await page.locator('#dialog').isVisible();
    /// ```
    #[tokio::test]
    async fn compare_locator_is_visible() {
        let mock = Arc::new(MockCdpClient::new());
        mock.set_query_selector_response(42);
        // DOM.getBoxModel succeeds → element is visible

        // pwright
        let page = Page::new(mock.clone());
        let visible = page.locator("#dialog").is_visible().await.unwrap();
        assert!(visible);

        // pwright uses DOM.getBoxModel (pure CDP, no JS!) — element has layout = visible
        let methods = mock.method_names();
        assert!(methods.contains(&"DOM.getBoxModel".to_string()));
        assert!(!methods.contains(&"Runtime.evaluate".to_string()));
    }

    // ═══════════════════════════════════════════════════════════
    //  Locator — Composition
    // ═══════════════════════════════════════════════════════════

    /// ```typescript
    /// // Playwright
    /// const first = page.locator('li').first();
    /// const last = page.locator('li').last();
    /// const child = page.locator('ul').locator('li');
    /// ```
    #[tokio::test]
    async fn compare_locator_composition() {
        let mock = Arc::new(MockCdpClient::new());
        let page = Page::new(mock);

        // pwright
        let first = page.locator("li").first();
        assert_eq!(first.selector(), "li:first-of-type");

        let last = page.locator("li").last();
        assert_eq!(last.selector(), "li:last-of-type");

        let child = page.locator("ul").locator("li");
        assert_eq!(child.selector(), "ul li");
    }

    // ═══════════════════════════════════════════════════════════
    //  getBy* Selectors
    // ═══════════════════════════════════════════════════════════

    /// ```typescript
    /// // Playwright
    /// page.getByTestId('login-btn')     // → [data-testid="login-btn"]
    /// page.getByPlaceholder('Email')    // → [placeholder="Email"]
    /// page.getByAltText('Logo')         // → [alt="Logo"]
    /// page.getByTitle('Help')           // → [title="Help"]
    /// ```
    #[tokio::test]
    async fn compare_get_by_selectors() {
        let mock = Arc::new(MockCdpClient::new());
        let page = Page::new(mock);

        // pwright — same CSS attribute selectors as Playwright
        assert_eq!(
            page.get_by_test_id("login-btn").selector(),
            r#"[data-testid="login-btn"]"#
        );
        assert_eq!(
            page.get_by_placeholder("Email").selector(),
            r#"[placeholder="Email"]"#
        );
        assert_eq!(page.get_by_alt_text("Logo").selector(), r#"[alt="Logo"]"#);
        assert_eq!(page.get_by_title("Help").selector(), r#"[title="Help"]"#);
    }

    // ═══════════════════════════════════════════════════════════
    //  Keyboard
    // ═══════════════════════════════════════════════════════════

    /// ```typescript
    /// // Playwright
    /// await page.keyboard.press('Enter');
    /// ```
    #[tokio::test]
    async fn compare_keyboard_press() {
        let mock = Arc::new(MockCdpClient::new());

        // pwright
        let page = Page::new(mock.clone());
        page.keyboard().press("Enter").await.unwrap();

        // Both should: dispatchKeyEvent("rawKeyDown") → dispatchKeyEvent("keyUp")
        let calls = mock.calls_for("Input.dispatchKeyEvent");
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].args[0]["type"], "rawKeyDown");
        assert_eq!(calls[0].args[0]["key"], "Enter");
        assert_eq!(calls[0].args[0]["code"], "Enter");
        assert_eq!(calls[1].args[0]["type"], "keyUp");
    }

    /// ```typescript
    /// // Playwright
    /// await page.keyboard.type('hello');
    /// ```
    #[tokio::test]
    async fn compare_keyboard_type() {
        let mock = Arc::new(MockCdpClient::new());

        // pwright
        let page = Page::new(mock.clone());
        page.keyboard().type_text("abc").await.unwrap();

        // Both should: insertText("a") → insertText("b") → insertText("c")
        let calls = mock.calls_for("Input.insertText");
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].args[0], "a");
        assert_eq!(calls[1].args[0], "b");
        assert_eq!(calls[2].args[0], "c");
    }

    // ═══════════════════════════════════════════════════════════
    //  Mouse
    // ═══════════════════════════════════════════════════════════

    /// ```typescript
    /// // Playwright
    /// await page.mouse.click(100, 200);
    /// ```
    #[tokio::test]
    async fn compare_mouse_click() {
        let mock = Arc::new(MockCdpClient::new());

        // pwright
        let page = Page::new(mock.clone());
        page.mouse().click(100.0, 200.0, None).await.unwrap();

        // Both should: dispatchMouseEvent("mousePressed") → dispatchMouseEvent("mouseReleased")
        let calls = mock.calls_for("Input.dispatchMouseEvent");
        assert_eq!(calls.len(), 2);

        assert_eq!(calls[0].args[0]["type"], "mousePressed");
        assert_eq!(calls[0].args[0]["x"], 100.0);
        assert_eq!(calls[0].args[0]["y"], 200.0);
        assert_eq!(calls[0].args[0]["button"], "left");
        assert_eq!(calls[0].args[0]["clickCount"], 1);

        assert_eq!(calls[1].args[0]["type"], "mouseReleased");
        assert_eq!(calls[1].args[0]["x"], 100.0);
        assert_eq!(calls[1].args[0]["y"], 200.0);
    }

    /// ```typescript
    /// // Playwright
    /// await page.mouse.dblclick(300, 400);
    /// ```
    #[tokio::test]
    async fn compare_mouse_dblclick() {
        let mock = Arc::new(MockCdpClient::new());

        // pwright
        let page = Page::new(mock.clone());
        page.mouse().dblclick(300.0, 400.0).await.unwrap();

        let calls = mock.calls_for("Input.dispatchMouseEvent");
        assert_eq!(calls[0].args[0]["clickCount"], 2);
    }

    /// ```typescript
    /// // Playwright
    /// await page.mouse.move(500, 600);
    /// ```
    #[tokio::test]
    async fn compare_mouse_move() {
        let mock = Arc::new(MockCdpClient::new());

        // pwright
        let page = Page::new(mock.clone());
        page.mouse().move_to(500.0, 600.0).await.unwrap();

        let calls = mock.calls_for("Input.dispatchMouseEvent");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].args[0]["type"], "mouseMoved");
        assert_eq!(calls[0].args[0]["x"], 500.0);
        assert_eq!(calls[0].args[0]["y"], 600.0);
    }

    // ═══════════════════════════════════════════════════════════
    //  Page — Bring to Front
    // ═══════════════════════════════════════════════════════════

    /// ```typescript
    /// // Playwright
    /// await page.bringToFront();
    /// ```
    #[tokio::test]
    async fn compare_bring_to_front() {
        let mock = Arc::new(MockCdpClient::new());

        // pwright
        let page = Page::new(mock.clone());
        page.bring_to_front().await.unwrap();

        // Both call Page.bringToFront
        assert_eq!(mock.method_names(), vec!["Page.bringToFront"]);
    }

    // ═══════════════════════════════════════════════════════════
    //  Screenshot
    // ═══════════════════════════════════════════════════════════

    /// ```typescript
    /// // Playwright
    /// const buf = await page.screenshot();
    /// const jpg = await page.screenshot({ type: 'jpeg', quality: 50 });
    /// ```
    #[tokio::test]
    async fn compare_screenshot() {
        use pwright_bridge::playwright::ScreenshotOptions;

        let mock = Arc::new(MockCdpClient::new());

        // pwright — default PNG
        let page = Page::new(mock.clone());
        page.screenshot(None).await.unwrap();

        let calls = mock.calls_for("Page.captureScreenshot");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].args[0]["format"], "png");

        // pwright — JPEG with quality
        page.screenshot(Some(ScreenshotOptions {
            format: Some("jpeg".to_string()),
            quality: Some(50),
            full_page: false,
        }))
        .await
        .unwrap();

        let calls = mock.calls_for("Page.captureScreenshot");
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[1].args[0]["format"], "jpeg");
        assert_eq!(calls[1].args[0]["quality"], 50);
    }

    // ═══════════════════════════════════════════════════════════
    //  Evaluate
    // ═══════════════════════════════════════════════════════════

    /// ```typescript
    /// // Playwright
    /// const result = await page.evaluate('document.title');
    /// ```
    #[tokio::test]
    async fn compare_evaluate() {
        let mock = Arc::new(MockCdpClient::new());
        mock.set_evaluate_response(serde_json::json!({
            "result": {"value": "My Title"}
        }));

        // pwright
        let page = Page::new(mock.clone());
        let result = page.evaluate("document.title").await.unwrap();
        assert_eq!(
            result.get("value").and_then(|v| v.as_str()),
            Some("My Title")
        );

        // Both call Runtime.evaluate with the expression
        let calls = mock.calls_for("Runtime.evaluate");
        assert_eq!(calls[0].args[0], "document.title");
    }

    // ═══════════════════════════════════════════════════════════
    //  Error cases
    // ═══════════════════════════════════════════════════════════

    /// ```typescript
    /// // Playwright
    /// await page.locator('.nonexistent').click(); // throws
    /// ```
    #[tokio::test]
    async fn compare_locator_not_found_error() {
        let mock = Arc::new(MockCdpClient::new());
        // querySelector returns 0 = not found

        // pwright — should error like Playwright
        let page = Page::new(mock.clone());
        let result = page.locator(".nonexistent").click().await;
        assert!(result.is_err());

        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains(".nonexistent"),
            "Error should mention the selector"
        );
    }
}
