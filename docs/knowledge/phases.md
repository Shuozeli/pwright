# Implementation Phases

Phased rollout of Playwright-compatible API for pwright. Each phase is self-contained — the library is usable after every phase, each phase adds a broader API surface.

> **Constraint:** Remote CDP only. All JS snippets go in `pwright-js`. All new bridge code uses `&dyn CdpClient` for testability.

---

## Phase 0 — Foundation (current state)

Already shipped. This is what we have today.

### Crate structure

```
pwright-cdp        Low-level WebSocket + CDP protocol
pwright-js         Centralized JS snippets (7 snippets, 7 tests)
pwright-bridge     High-level browser ops (37 tests)
pwright-server     gRPC service layer
```

### Existing CdpClient trait (27 methods)

| Domain | Methods |
|--------|---------|
| Page | `page_navigate`, `page_enable`, `page_capture_screenshot`, `page_print_to_pdf`, `page_add_script_on_new_document` |
| DOM | `dom_focus`, `dom_scroll_into_view`, `dom_get_box_model`, `dom_resolve_node`, `dom_enable`, `dom_get_document` |
| Input | `input_dispatch_mouse_event`, `input_dispatch_key_event`, `input_insert_text` |
| Runtime | `runtime_evaluate`, `runtime_call_function_on`, `runtime_enable` |
| Accessibility | `accessibility_get_full_tree` |
| Network | `network_enable`, `network_set_blocked_urls`, `network_get_cookies`, `network_set_cookies` |
| Fetch | `fetch_enable`, `fetch_disable`, `fetch_continue_request`, `fetch_fail_request` |
| Target | `target_create`, `target_close`, `target_get_targets`, `target_attach`, `target_detach` |

### Existing bridge functions

| Module | Functions |
|--------|----------|
| `navigate` | `navigate(session, url, opts)` with wait strategies |
| `actions` | `click_by_node_id`, `type_by_node_id`, `fill_by_node_id`, `hover_by_node_id`, `select_by_node_id`, `scroll_by_node_id`, `scroll_page`, `drag_by_node_id`, `press_key`, `focus_element` |
| `content` | `take_screenshot`, `get_pdf`, `get_text` |
| `snapshot` | `build_snapshot` (accessibility tree → A11yNode tree) |
| `evaluate` | `evaluate(session, expr)` |
| `cookies` | `get_cookies`, `set_cookies` |

---

## Phase 1 — Page + Locator + Keyboard + Mouse

**Goal:** A Playwright user can navigate, find elements, click, type, and take screenshots using familiar API.

### New files

```
crates/pwright-bridge/src/playwright/
├── mod.rs              Re-exports
├── page.rs             Page struct
├── locator.rs          Locator + CSS selector resolution
├── keyboard.rs         Keyboard struct
├── mouse.rs            Mouse struct
└── selectors.rs        Selector parsing (css=, text=, etc.)
```

### New CDP methods needed on CdpClient

| Method | CDP command | Why |
|--------|------------|-----|
| `dom_query_selector(node_id, selector)` | `DOM.querySelector` | Locator resolution |
| `dom_query_selector_all(node_id, selector)` | `DOM.querySelectorAll` | `locator.count()`, `locator.all()` |
| `dom_get_attributes(node_id)` | `DOM.getAttributes` | `locator.get_attribute()` (no JS) |
| `dom_get_outer_html(node_id)` | `DOM.getOuterHTML` | `locator.inner_html()` |
| `page_reload()` | `Page.reload` | `page.reload()` |
| `page_get_navigation_history()` | `Page.getNavigationHistory` | `page.go_back()` / `page.go_forward()` |
| `page_navigate_to_history_entry(id)` | `Page.navigateToHistoryEntry` | History navigation |

### New pwright-js snippets

| Snippet | Module | Used by |
|---------|--------|---------|
| `GET_TEXT_CONTENT` | `element` | `locator.text_content()` |
| `GET_INNER_TEXT` (element) | `element` | `locator.inner_text()` |
| `GET_INPUT_VALUE` | `element` | `locator.input_value()` |

### Page API (this phase)

```rust
let page = browser.new_page().await?;

// Navigation
page.goto("https://example.com", None).await?;
page.reload(None).await?;
page.go_back(None).await?;
page.go_forward(None).await?;
page.url();
page.title().await?;
page.content().await?;
page.close().await?;
page.is_closed();

// Screenshots / PDF
page.screenshot(ScreenshotOpts::default()).await?;
page.pdf(PdfOpts::default()).await?;

// Evaluation
page.evaluate("document.title").await?;

// Locators
page.locator("button.submit").click(None).await?;
page.locator("#email").fill("user@example.com").await?;
page.locator("input").type_text("hello", None).await?;
page.locator("form").press("Enter").await?;

// Input devices
page.keyboard().press("Tab").await?;
page.keyboard().type_text("hello").await?;
page.mouse().click(100.0, 200.0, None).await?;
page.mouse().wheel(0.0, 500.0).await?;
```

### Locator API (this phase)

```rust
let loc = page.locator("button.submit");

// Actions
loc.click(None).await?;
loc.fill("value").await?;
loc.type_text("text", None).await?;
loc.press("Enter").await?;
loc.hover(None).await?;
loc.focus().await?;
loc.check().await?;
loc.uncheck().await?;
loc.select_option("value").await?;
loc.scroll_into_view().await?;

// Queries
loc.text_content().await?;   // JS via pwright-js
loc.inner_text().await?;     // JS via pwright-js
loc.inner_html().await?;     // DOM.getOuterHTML (no JS)
loc.get_attribute("href").await?;  // DOM.getAttributes (no JS)
loc.input_value().await?;    // JS via pwright-js
loc.bounding_box().await?;   // DOM.getBoxModel (no JS)
loc.is_visible().await?;     // DOM.getBoxModel (no JS)
loc.is_checked().await?;     // AX tree (no JS)
loc.is_enabled().await?;     // AX tree (no JS)
loc.count().await?;          // querySelectorAll (no JS)
loc.wait_for(None).await?;   // Poll querySelector
loc.screenshot(None).await?; // Clip screenshot

// Composition
loc.first();
loc.last();
loc.nth(2);
loc.locator("span.icon");   // Scoped sub-selector
```

### Tests (target: ~25 new tests)

| Test area | Count | Strategy |
|-----------|-------|----------|
| Page navigation | 5 | MockCdpClient |
| Locator resolution | 5 | Mock DOM.querySelector |
| Locator actions | 8 | Mock click/fill/type delegation |
| Locator queries | 4 | Mock getAttributes, getBoxModel |
| Keyboard | 3 | Mock Input.dispatchKeyEvent |

---

## Phase 2 — getBy* Queries + BrowserContext

**Goal:** Semantic element selection and cookie/permission management.

### New files

```
crates/pwright-bridge/src/playwright/
├── context.rs          BrowserContext struct
└── selectors.rs        ← extend with getBy* resolvers
```

### getBy* selector strategies

| Query | Resolution | Needs JS? |
|-------|-----------|-----------|
| `get_by_role("button")` | AX tree filter by role | No |
| `get_by_text("Submit")` | AX tree filter by name | No |
| `get_by_test_id("login")` | CSS `[data-testid="login"]` | No |
| `get_by_placeholder("Email")` | CSS `[placeholder="Email"]` | No |
| `get_by_alt_text("Logo")` | CSS `[alt="Logo"]` | No |
| `get_by_title("Help")` | CSS `[title="Help"]` | No |
| `get_by_label("Email")` | AX tree `labelledby` or `<label>` walk | No |

> **Key insight:** All `getBy*` queries can be implemented JS-free using the accessibility tree or CSS attribute selectors.

### BrowserContext API

```rust
let context = browser.new_context().await?;

// Pages
let page = context.new_page().await?;
context.pages();

// Cookies
context.cookies(None).await?;
context.add_cookies(cookies).await?;
context.clear_cookies().await?;

// Configuration
context.set_default_timeout(30_000);
context.set_default_navigation_timeout(60_000);

// Permissions & Emulation
context.grant_permissions(vec!["geolocation"]).await?;
context.clear_permissions().await?;
context.set_geolocation(Some(Geolocation { ... })).await?;
context.set_extra_http_headers(headers).await?;
context.set_offline(true).await?;

context.close().await?;
```

### New CDP methods needed

| Method | CDP command |
|--------|------------|
| `network_clear_browser_cookies()` | `Network.clearBrowserCookies` |
| `browser_grant_permissions(perms)` | `Browser.grantPermissions` |
| `browser_reset_permissions()` | `Browser.resetPermissions` |
| `emulation_set_geolocation(geo)` | `Emulation.setGeolocationOverride` |
| `network_set_extra_http_headers(h)` | `Network.setExtraHTTPHeaders` |
| `network_emulate_conditions(opts)` | `Network.emulateNetworkConditions` |

### Waiting APIs

```rust
page.wait_for_load_state(LoadState::NetworkIdle).await?;
page.wait_for_url("**/dashboard").await?;
page.wait_for_timeout(1000).await?;
```

### Tests (target: ~20 new tests)

| Test area | Count |
|-----------|-------|
| getByRole resolution | 4 |
| getByText resolution | 3 |
| getByTestId, Placeholder, etc. | 5 |
| BrowserContext cookies | 3 |
| BrowserContext permissions | 2 |
| waitFor* | 3 |

---

## Phase 3 — Frame + Touchscreen + Event System

**Goal:** Page events, iframe support, and touch input.

### New files

```
crates/pwright-bridge/src/playwright/
├── frame.rs            Frame struct (main frame + iframes)
├── touchscreen.rs      Touchscreen struct
└── events.rs           Event emitter for page events
```

### Frame support

```rust
let main = page.main_frame();
let frames = page.frames();
let named = page.frame("iframe-name");

// Frames share the same API as Page for content/actions
main.url();
main.title().await?;
main.locator("button").click(None).await?;
```

### CDP additions for iframes

| Method | CDP command |
|--------|------------|
| `target_set_discover_targets(discover)` | `Target.setDiscoverTargets` |
| `target_attach_to_target(id, flatten)` | `Target.attachToTarget` (flatten for OOPIFs) |
| `runtime_get_properties(object_id)` | `Runtime.getProperties` |

### Page event system

```rust
// Event listener pattern
page.on_console(|msg| println!("console: {}", msg)).await;
page.on_dialog(|dialog| dialog.accept(None)).await;
page.on_request(|req| println!(">> {}", req.url())).await;
page.on_response(|res| println!("<< {} {}", res.status(), res.url())).await;

// Wait for events
page.wait_for_request("**/api/**").await?;
page.wait_for_response("**/api/**").await?;
page.wait_for_event("dialog").await?;
```

### CDP event subscriptions needed

| Event | CDP event |
|-------|-----------|
| Console | `Runtime.consoleAPICalled` |
| Dialog | `Page.javascriptDialogOpening` |
| Request | `Network.requestWillBeSent` |
| Response | `Network.responseReceived` |
| Download | `Page.downloadWillBegin` |

### Touchscreen

```rust
page.touchscreen().tap(100.0, 200.0).await?;
```

| Method | CDP command |
|--------|------------|
| `input_dispatch_touch_event(type, x, y)` | `Input.dispatchTouchEvent` |

### Tests (target: ~15 new tests)

| Test area | Count |
|-----------|-------|
| Frame resolution | 3 |
| Frame actions | 3 |
| Event emitter | 4 |
| Dialog handling | 2 |
| Touchscreen | 2 |
| Console capture | 1 |

---

## Phase 4 — Request Interception + Advanced Locator Composition

**Goal:** Full network control and complex element queries.

### Request interception

```rust
// Intercept and modify requests
page.route("**/*.png", |route| {
    route.abort().await
}).await?;

page.route("**/api/**", |route| {
    route.fulfill(200, headers, body).await
}).await?;

page.route("**/api/**", |route| {
    let request = route.request();
    // Modify and continue
    route.continue_with(modified_headers).await
}).await?;

page.unroute("**/*.png").await?;
```

### CDP: Fetch domain integration

The `CdpClient` already has `fetch_enable`, `fetch_disable`, `fetch_continue_request`, `fetch_fail_request`. Additions needed:

| Method | CDP command |
|--------|------------|
| `fetch_fulfill_request(id, status, headers, body)` | `Fetch.fulfillRequest` |
| `fetch_get_response_body(id)` | `Fetch.getResponseBody` |

### Advanced Locator

```rust
// Composition
let button = page.locator("button");
let submit = button.filter(LocatorFilter::has_text("Submit"));
let primary = button.and(page.locator(".primary"));
let any = page.locator("button").or(page.locator("a.button"));

// Evaluation
loc.evaluate("el => el.dataset.count").await?;
loc.evaluate_all("els => els.length").await?;

// File input
loc.set_input_files(vec!["path/to/file.txt"]).await?;

// Drag
loc.drag_to(target_locator).await?;
```

### CDP additions

| Method | CDP command |
|--------|------------|
| `dom_set_file_input_files(node_id, files)` | `DOM.setFileInputFiles` |
| `runtime_add_binding(name)` | `Runtime.addBinding` |
| `page_handle_dialog(accept, text)` | `Page.handleJavaScriptDialog` |

### Tests (target: ~15 new tests)

| Test area | Count |
|-----------|-------|
| Route/abort | 3 |
| Route/fulfill | 3 |
| Route/continue | 2 |
| Locator filter | 3 |
| Locator and/or | 2 |
| File input | 1 |
| exposeFunction | 1 |

---

## Summary

| Phase | New files | New CDP methods | New tests | Cumulative tests |
|-------|-----------|----------------|-----------|-----------------|
| 0 (done) | — | — | 44 | 44 |
| 1 | 6 | 7 | ~25 | ~69 |
| 2 | 1 (+extend) | 6 | ~20 | ~89 |
| 3 | 3 | 5 | ~15 | ~104 |
| 4 | 0 (+extend) | 3 | ~15 | ~119 |

### gRPC Service Updates

Each phase also extends the gRPC proto with corresponding RPCs. The `pwright-server` service handlers will delegate to the new `playwright/` module methods, keeping the service layer thin.
