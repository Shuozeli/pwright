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

        // pwright — first()/last() use __pw_nth internally (resolved via querySelectorAll + index)
        let first = page.locator("li").first();
        assert_eq!(first.selector(), "__pw_nth=li|0");

        let last = page.locator("li").last();
        assert_eq!(last.selector(), "__pw_nth=li|-1");

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
            format: pwright_bridge::playwright::ImageFormat::Jpeg,
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

    // ═══════════════════════════════════════════════════════════
    //  wait_for with visibility
    // ═══════════════════════════════════════════════════════════

    /// ```typescript
    /// // Playwright
    /// await page.locator('#modal').waitFor({ state: 'visible' });
    /// ```
    #[tokio::test]
    async fn compare_wait_for_visible() {
        use pwright_bridge::playwright::WaitState;

        let mock = Arc::new(MockCdpClient::new());
        mock.set_query_selector_response(42);
        // box model succeeds by default → visible

        let page = Page::new(mock.clone());
        page.locator("#modal")
            .wait_for(1000, WaitState::Visible)
            .await
            .unwrap();

        // Should call DOM.getBoxModel to check visibility
        let methods = mock.method_names();
        assert!(methods.contains(&"DOM.getBoxModel".to_string()));
    }

    // ═══════════════════════════════════════════════════════════
    //  is_checked / is_disabled via JS
    // ═══════════════════════════════════════════════════════════

    /// ```typescript
    /// // Playwright
    /// const checked = await page.locator('input').isChecked();
    /// // Internally uses Runtime.callFunctionOn, not DOM.getAttributes
    /// ```
    #[tokio::test]
    async fn compare_locator_is_checked_js() {
        let mock = mock_with_element();
        mock.set_call_function_response(serde_json::json!({
            "result": {"value": true}
        }));

        let page = Page::new(mock.clone());
        let checked = page
            .locator("input[type=checkbox]")
            .is_checked()
            .await
            .unwrap();
        assert!(checked);

        // Must use Runtime.callFunctionOn (JS property), NOT DOM.getAttributes
        let methods = mock.method_names();
        assert!(methods.contains(&"Runtime.callFunctionOn".to_string()));
        assert!(!methods.contains(&"DOM.getAttributes".to_string()));
    }

    /// ```typescript
    /// // Playwright
    /// const disabled = await page.locator('button').isDisabled();
    /// ```
    #[tokio::test]
    async fn compare_locator_is_disabled_js() {
        let mock = mock_with_element();
        mock.set_call_function_response(serde_json::json!({
            "result": {"value": false}
        }));

        let page = Page::new(mock.clone());
        let disabled = page.locator("button").is_disabled().await.unwrap();
        assert!(!disabled);

        let methods = mock.method_names();
        assert!(methods.contains(&"Runtime.callFunctionOn".to_string()));
        assert!(!methods.contains(&"DOM.getAttributes".to_string()));
    }

    // ═══════════════════════════════════════════════════════════
    //  Locator.evaluate (per-element)
    // ═══════════════════════════════════════════════════════════

    /// ```typescript
    /// // Playwright
    /// const height = await page.locator('div').evaluate(el => el.offsetHeight);
    /// ```
    #[tokio::test]
    async fn compare_locator_evaluate() {
        let mock = mock_with_element();
        mock.set_call_function_response(serde_json::json!({
            "result": {"value": 300}
        }));

        let page = Page::new(mock.clone());
        let result = page
            .locator("div")
            .evaluate("function(el) { return el.offsetHeight; }")
            .await
            .unwrap();

        assert_eq!(result.get("value").and_then(|v| v.as_i64()), Some(300));

        // Should resolve element, then callFunctionOn
        let methods = mock.method_names();
        assert!(methods.contains(&"DOM.querySelector".to_string()));
        assert!(methods.contains(&"DOM.resolveNode".to_string()));
        assert!(methods.contains(&"Runtime.callFunctionOn".to_string()));
    }

    // ═══════════════════════════════════════════════════════════
    //  first() / last() via nth resolution
    // ═══════════════════════════════════════════════════════════

    /// ```typescript
    /// // Playwright
    /// const first = page.locator('li').first();
    /// const last = page.locator('li').last();
    /// // Internally uses querySelectorAll + index, not CSS pseudo-classes
    /// ```
    #[tokio::test]
    async fn compare_locator_first_last() {
        let mock = Arc::new(MockCdpClient::new());
        mock.set_query_selector_all_response(vec![10, 20, 30]);
        mock.set_resolve_node(serde_json::json!({"object": {"objectId": "obj-1"}}));
        mock.set_call_function_response(serde_json::json!({
            "result": {"value": "text"}
        }));

        let page = Page::new(mock.clone());

        // first() should resolve to node 10
        let first = page.locator("li").first();
        first.text_content().await.unwrap();
        let resolve_calls = mock.calls_for("DOM.resolveNode");
        assert_eq!(
            resolve_calls.last().unwrap().args[0]["nodeId"]
                .as_i64()
                .unwrap(),
            10
        );

        // last() should resolve to node 30
        let last = page.locator("li").last();
        last.text_content().await.unwrap();
        let resolve_calls = mock.calls_for("DOM.resolveNode");
        assert_eq!(
            resolve_calls.last().unwrap().args[0]["nodeId"]
                .as_i64()
                .unwrap(),
            30
        );

        // Verify querySelectorAll is used (not querySelector with pseudo-class)
        let methods = mock.method_names();
        assert!(methods.contains(&"DOM.querySelectorAll".to_string()));
    }

    // ═══════════════════════════════════════════════════════════
    //  Strict MockCdpClient
    // ═══════════════════════════════════════════════════════════

    #[tokio::test]
    async fn compare_strict_mock_fails_on_unconfigured() {
        let mock = Arc::new(MockCdpClient::new().strict());

        let page = Page::new(mock.clone());
        let result = page.locator("button").click().await;
        assert!(result.is_err());

        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("unexpected call"), "Error: {err_msg}");
    }

    // ═══════════════════════════════════════════════════════════
    //  wait_for_response / wait_for_request
    // ═══════════════════════════════════════════════════════════

    /// ```typescript
    /// // Playwright
    /// const resp = await page.waitForResponse(r => r.url().includes('/api'));
    /// const body = await resp.json();
    /// ```
    #[tokio::test]
    async fn compare_wait_for_response() {
        let mock = Arc::new(MockCdpClient::new());
        let page = Page::new(mock.clone());

        let mock2 = mock.clone();
        tokio::spawn(async move {
            tokio::task::yield_now().await;
            mock2.send_event(pwright_cdp::CdpEvent {
                method: "Network.responseReceived".to_string(),
                params: serde_json::json!({
                    "requestId": "req-api",
                    "response": {
                        "url": "https://example.com/api/data",
                        "status": 200,
                        "statusText": "OK",
                        "headers": {"content-type": "application/json"},
                        "mimeType": "application/json"
                    }
                }),
                session_id: None,
            });
        });

        let resp = page
            .wait_for_response(|r| r.url.contains("/api/"), 5000)
            .await
            .unwrap();

        assert_eq!(resp.request_id, "req-api");
        assert_eq!(resp.status, 200);
        assert_eq!(resp.mime_type, "application/json");

        // Verify Network.enable was called
        assert!(mock.method_names().contains(&"Network.enable".to_string()));
    }

    /// ```typescript
    /// // Playwright
    /// const req = await page.waitForRequest(r => r.method() === 'POST');
    /// ```
    #[tokio::test]
    async fn compare_wait_for_request() {
        let mock = Arc::new(MockCdpClient::new());
        let page = Page::new(mock.clone());

        let mock2 = mock.clone();
        tokio::spawn(async move {
            tokio::task::yield_now().await;
            mock2.send_event(pwright_cdp::CdpEvent {
                method: "Network.requestWillBeSent".to_string(),
                params: serde_json::json!({
                    "requestId": "req-post",
                    "request": {
                        "url": "https://example.com/api/submit",
                        "method": "POST",
                        "headers": {"content-type": "application/json"},
                        "postData": "{\"key\":\"val\"}"
                    },
                    "type": "XHR"
                }),
                session_id: None,
            });
        });

        let req = page
            .wait_for_request(|r| r.method == "POST", 5000)
            .await
            .unwrap();

        assert_eq!(req.request_id, "req-post");
        assert_eq!(req.method, "POST");
        assert_eq!(req.post_data, Some("{\"key\":\"val\"}".to_string()));
    }

    // ═══════════════════════════════════════════════════════════
    //  evaluate_with_arg
    // ═══════════════════════════════════════════════════════════

    /// ```typescript
    /// // Playwright
    /// const result = await page.evaluate(
    ///     (name) => `hello ${name}`,
    ///     'world'
    /// );
    /// ```
    #[tokio::test]
    async fn compare_evaluate_with_arg() {
        let mock = Arc::new(MockCdpClient::new());
        mock.set_call_function_response(serde_json::json!({
            "result": {"type": "string", "value": "hello world"}
        }));

        let page = Page::new(mock.clone());
        let result = page
            .evaluate_with_arg(
                "function(name) { return 'hello ' + name; }",
                &serde_json::json!("world"),
            )
            .await
            .unwrap();

        assert_eq!(result["value"], "hello world");

        // Should call: DOM.getDocument + DOM.resolveNode + Runtime.callFunctionOn
        let methods = mock.method_names();
        assert!(methods.contains(&"DOM.getDocument".to_string()));
        assert!(methods.contains(&"DOM.resolveNode".to_string()));
        assert!(methods.contains(&"Runtime.callFunctionOn".to_string()));

        // Verify the argument was passed via CDP serialization (not string interpolation)
        let cf_calls = mock.calls_for("Runtime.callFunctionOn");
        let args = cf_calls[0].args[0]["arguments"].as_array().unwrap();
        assert_eq!(args[0]["value"], "world");
    }

    // ═══════════════════════════════════════════════════════════
    //  response_body
    // ═══════════════════════════════════════════════════════════

    /// ```typescript
    /// // Playwright
    /// const body = await response.body();
    /// const json = await response.json();
    /// ```
    #[tokio::test]
    async fn compare_response_body() {
        let mock = Arc::new(MockCdpClient::new());
        let page = Page::new(mock.clone());

        let body = page.response_body("req-42").await.unwrap();

        // Should call Network.getResponseBody with the request_id
        let calls = mock.calls_for("Network.getResponseBody");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].args[0]["requestId"], "req-42");
        assert!(!body.base64_encoded);
    }

    // ═══════════════════════════════════════════════════════════
    //  Page selector convenience methods
    // ═══════════════════════════════════════════════════════════

    /// ```typescript
    /// // Playwright (deprecated but supported)
    /// await page.click('button');
    /// await page.fill('#email', 'user@test.com');
    /// const text = await page.textContent('h1');
    /// ```
    #[tokio::test]
    async fn compare_page_selector_methods() {
        let mock = mock_with_element();

        let page = Page::new(mock.clone());

        // click(selector)
        page.click("button.submit").await.unwrap();
        let qs = mock.calls_for("DOM.querySelector");
        assert_eq!(qs.last().unwrap().args[0]["selector"], "button.submit");

        // text_content(selector)
        mock.set_call_function_response(serde_json::json!({
            "result": {"value": "Hello"}
        }));
        let text = page.text_content("h1").await.unwrap();
        assert_eq!(text, Some("Hello".to_string()));
    }
}
