# Playwright API Compatibility

pwright aims to provide a Rust API with the same mental model as [Playwright](https://playwright.dev). This document maps every Playwright public API to its pwright status: **supported**, **planned**, or **out of scope**.

> **Constraint:** pwright only supports **remote CDP** — connecting to an externally-managed Chrome via WebSocket. No browser launch, no profiles, no tracing, no video.

---

## Object Model

```
Playwright
  └─ BrowserType.connectOverCDP(wsEndpoint)  ← only entry point
       └─ Browser
            └─ BrowserContext
                 └─ Page
                      ├─ Frame (main + iframes)
                      │    └─ Locator
                      ├─ Keyboard
                      ├─ Mouse
                      └─ Touchscreen
```

| Playwright class | pwright Rust struct | Status |
|-----------------|-------------------|--------|
| `BrowserType` | — | Only `connectOverCDP()` via `Browser::connect()` |
| `Browser` | `Browser` | ✅ Exists |
| `BrowserContext` | `Context` | Planned (Tier 2) |
| `Page` | `Page` | ✅ Exists |
| `Frame` | `Frame` | Planned (Tier 3) |
| `Locator` | `Locator` | ✅ Exists |
| `Keyboard` | `Keyboard` | ✅ Exists |
| `Mouse` | `Mouse` | ✅ Exists |
| `Touchscreen` | `Touchscreen` | ✅ Exists |

---

## Browser

| Playwright API | pwright | Status | Notes |
|---------------|---------|--------|-------|
| `browser.newContext(opts)` | `browser.new_context()` | Planned | Single default context for remote CDP |
| `browser.newPage(opts)` | `browser.new_page()` | Planned | Shorthand: creates context + page |
| `browser.contexts()` | `browser.contexts()` | Planned | Returns list of contexts |
| `browser.isConnected()` | `browser.is_connected()` | Planned | Track WebSocket state |
| `browser.version()` | `browser.version()` | Planned | `Browser.getVersion` CDP |
| `browser.close()` | `browser.close()` | Planned | Close WebSocket connection |
| `browser.browserType()` | — | ❌ Out of scope | |
| `browser.newBrowserCDPSession()` | — | ❌ Out of scope | |
| `browser.startTracing()` | — | ❌ Out of scope | |
| `browser.stopTracing()` | — | ❌ Out of scope | |

---

## BrowserContext

| Playwright API | pwright | Status | CDP method |
|---------------|---------|--------|------------|
| `context.newPage()` | `context.new_page()` | Planned | `Target.createTarget` |
| `context.pages()` | `context.pages()` | Planned | List open tabs |
| `context.cookies(urls)` | `context.cookies()` | ✅ Exists | `Network.getCookies` |
| `context.addCookies(cookies)` | `context.add_cookies()` | ✅ Exists | `Network.setCookie` |
| `context.clearCookies()` | `context.clear_cookies()` | Planned | `Network.clearBrowserCookies` |
| `context.close()` | `context.close()` | Planned | Close all pages |
| `context.setDefaultTimeout(ms)` | `context.set_default_timeout()` | Planned | Client-side config |
| `context.setDefaultNavigationTimeout(ms)` | `context.set_default_navigation_timeout()` | Planned | Client-side config |
| `context.grantPermissions(perms)` | `context.grant_permissions()` | Planned | `Browser.grantPermissions` |
| `context.clearPermissions()` | `context.clear_permissions()` | Planned | `Browser.resetPermissions` |
| `context.setGeolocation(geo)` | `context.set_geolocation()` | Planned | `Emulation.setGeolocationOverride` |
| `context.setExtraHTTPHeaders(h)` | `context.set_extra_http_headers()` | Planned | `Network.setExtraHTTPHeaders` |
| `context.setOffline(bool)` | `context.set_offline()` | Planned | `Network.emulateNetworkConditions` |
| `context.setHTTPCredentials(c)` | — | Planned | `Fetch.enable` with credentials |
| `context.addInitScript(script)` | `context.add_init_script()` | Planned | `Page.addScriptToEvaluateOnNewDocument` |
| `context.storageState()` | — | ❌ Out of scope | Requires local filesystem |
| `context.route(url, handler)` | — | Planned (Tier 3) | `Fetch.enable` + intercept |
| `context.unroute(url)` | — | Planned (Tier 3) | `Fetch.disable` |
| `context.exposeBinding()` | — | Planned (Tier 3) | `Runtime.addBinding` |
| `context.exposeFunction()` | — | Planned (Tier 3) | `Runtime.addBinding` |
| `context.tracing` | — | ❌ Out of scope | |
| `context.request` (APIRequestContext) | — | ❌ Out of scope | |

---

## Page

### Navigation

| Playwright API | pwright | Status | CDP method |
|---------------|---------|--------|------------|
| `page.goto(url, opts)` | `page.goto()` | ✅ Exists | `Page.navigate` |
| `page.reload(opts)` | `page.reload()` | ✅ Exists | `Page.reload` |
| `page.goBack(opts)` | `page.go_back()` | ✅ Exists | `Page.navigateToHistoryEntry` |
| `page.goForward(opts)` | `page.go_forward()` | ✅ Exists | `Page.navigateToHistoryEntry` |
| `page.waitForLoadState(s)` | `page.wait_for_load_state()` | Partial | Extends existing wait logic |
| `page.waitForNavigation(opts)` | `page.wait_for_navigation()` | Planned | Event-based |
| `page.waitForURL(url)` | `page.wait_for_url()` | Planned | Poll + events |

### Content & State

| Playwright API | pwright | Status | Implementation |
|---------------|---------|--------|----------------|
| `page.url()` | `page.url()` | ✅ Exists | `Runtime.evaluate` |
| `page.title()` | `page.title()` | ✅ Exists | `Runtime.evaluate` |
| `page.content()` | `page.content()` | ✅ Exists | `Runtime.evaluate` |
| `page.setContent(html)` | `page.set_content()` | Partial | CDP `Page.setDocumentContent` exists, no Page wrapper yet |
| `page.isClosed()` | `page.is_closed()` | ✅ Exists | Track state |
| `page.close()` | `page.close()` | ✅ Exists | `Target.closeTarget` via `TabHandle::close` |

### Evaluation

| Playwright API | pwright | Status | Implementation |
|---------------|---------|--------|----------------|
| `page.evaluate(fn, arg)` | `page.evaluate()` | ✅ Exists | `Runtime.evaluate` |
| `page.evaluate(fn, arg)` (async) | `page.evaluate_async()` | ✅ Exists | `Runtime.evaluate` with `awaitPromise: true` |
| `page.evaluateHandle(fn)` | `page.evaluate_handle()` | Planned | `Runtime.evaluate` `returnByValue:false` |
| `page.addInitScript(s)` | `page.add_init_script()` | Partial | CDP method exists, no Page wrapper yet |
| `page.addScriptTag(opts)` | `page.add_script_tag()` | Planned | JS eval inject `<script>` |
| `page.addStyleTag(opts)` | `page.add_style_tag()` | Planned | JS eval inject `<style>` |
| `page.exposeFunction()` | — | Planned (Tier 3) | `Runtime.addBinding` |
| `page.exposeBinding()` | — | Planned (Tier 3) | `Runtime.addBinding` |

### Screenshots & PDF

| Playwright API | pwright | Status | CDP method |
|---------------|---------|--------|------------|
| `page.screenshot(opts)` | `page.screenshot()` | ✅ Exists | `Page.captureScreenshot` |
| `page.pdf(opts)` | `page.pdf()` | ✅ Exists | `Page.printToPDF` |
| `page.snapshotForAI()` | bridge `get_snapshot()` | Partial | Available via bridge function, not on Page struct |

### Input (selector-based)

| Playwright API | pwright | Status | Notes |
|---------------|---------|--------|-------|
| `page.click(selector)` | `page.click()` | ✅ Exists | Via Locator |
| `page.dblclick(selector)` | `page.dblclick()` | ✅ Exists | `clickCount: 2` |
| `page.fill(selector, value)` | `page.fill()` | ✅ Exists | Via Locator |
| `page.type(selector, text)` | `page.type_text()` | ✅ Exists | Via Locator |
| `page.press(selector, key)` | `page.press()` | ✅ Exists | Via Locator |
| `page.hover(selector)` | `page.hover()` | ✅ Exists | Via Locator |
| `page.focus(selector)` | `page.focus()` | ✅ Exists | `DOM.focus` |
| `page.check(selector)` | `page.check()` | ✅ Exists | Click if unchecked |
| `page.uncheck(selector)` | `page.uncheck()` | ✅ Exists | Click if checked |
| `page.selectOption(selector, v)` | `page.select_option()` | ✅ Exists | DOM select override |
| `page.setInputFiles(sel, f)` | `page.set_input_files()` | ✅ Exists | `DOM.setFileInputFiles` |
| `page.tap(selector)` | `page.touchscreen().tap()` | ✅ Exists | `Input.dispatchTouchEvent` |
| `page.dragAndDrop(s, t)` | `page.drag_and_drop()` | Planned | Mouse event sequence |

### Locators (getBy)

| Playwright API | pwright | Status | Selector strategy |
|---------------|---------|--------|-------------------|
| `page.locator(selector)` | `page.locator()` | ✅ Exists | CSS `querySelector` |
| `page.getByRole(role)` | `page.get_by_role()` | ✅ Exists | JS implicit role mapping + `[role]` |
| `page.getByText(text)` | `page.get_by_text()` | ✅ Exists | JS text content matching |
| `page.getByLabel(text)` | `page.get_by_label()` | ✅ Exists | JS `<label>` + `aria-label` lookup |
| `page.getByPlaceholder(text)` | `page.get_by_placeholder()` | ✅ Exists | `[placeholder="..."]` CSS |
| `page.getByTestId(id)` | `page.get_by_test_id()` | ✅ Exists | `[data-testid="..."]` CSS |
| `page.getByAltText(text)` | `page.get_by_alt_text()` | ✅ Exists | `[alt="..."]` CSS |
| `page.getByTitle(text)` | `page.get_by_title()` | ✅ Exists | `[title="..."]` CSS |
| `page.frameLocator(sel)` | — | Planned (Tier 3) | iframe support |

### DOM Queries (selector-based)

| Playwright API | pwright | Status | Implementation |
|---------------|---------|--------|----------------|
| `page.textContent(sel)` | `page.text_content(sel)` | ✅ Exists | Delegates to `Locator::text_content()` |
| `page.innerText(sel)` | `page.inner_text(sel)` | ✅ Exists | Delegates to `Locator::inner_text()` |
| `page.innerHTML(sel)` | `page.inner_html(sel)` | ✅ Exists | Delegates to `Locator::inner_html()` |
| `page.getAttribute(sel, n)` | `page.get_attribute(sel, n)` | ✅ Exists | Delegates to `Locator::get_attribute()` |
| `page.inputValue(sel)` | `page.input_value()` | ✅ Exists | `Runtime.callFunctionOn` |
| `page.isChecked(sel)` | `page.is_checked()` | ✅ Exists | JS DOM property via `Runtime.callFunctionOn` |
| `page.isDisabled(sel)` | `page.is_disabled()` | ✅ Exists | JS DOM property via `Runtime.callFunctionOn` |
| `page.isEditable(sel)` | `page.is_editable()` | Planned | AX tree property (no JS) |
| `page.isEnabled(sel)` | `page.is_enabled()` | ✅ Exists | AX tree `disabled` inverse (no JS) |
| `page.isHidden(sel)` | `page.is_hidden()` | ✅ Exists | `DOM.getBoxModel` fails → hidden (no JS) |
| `page.isVisible(sel)` | `page.is_visible()` | ✅ Exists | `DOM.getBoxModel` succeeds → visible (no JS) |

### Emulation

| Playwright API | pwright | Status | CDP method |
|---------------|---------|--------|------------|
| `page.setViewportSize(size)` | `page.set_viewport_size()` | Planned | `Emulation.setDeviceMetricsOverride` |
| `page.viewportSize()` | `page.viewport_size()` | Planned | Cached locally |
| `page.emulateMedia(opts)` | `page.emulate_media()` | Planned | `Emulation.setEmulatedMedia` |

### Waiting

| Playwright API | pwright | Status | Implementation |
|---------------|---------|--------|----------------|
| `page.waitForTimeout(ms)` | `page.wait_for_timeout()` | ✅ Exists | `tokio::time::sleep` |
| `page.waitForFunction(fn)` | `page.wait_for_function()` | Planned (Tier 3) | Poll `Runtime.evaluate` |
| `page.waitForSelector(sel)` | `page.wait_for_selector()` | ✅ Exists | Poll `DOM.querySelector` |
| `page.waitForRequest(url)` | `page.on_request()` | ✅ Exists | `Network.requestWillBeSent` via mpsc channel |
| `page.waitForResponse(url)` | `page.on_response()` | ✅ Exists | `Network.responseReceived` via mpsc channel |
| `page.waitForEvent(ev)` | `page.wait_for_event()` | Planned (Tier 2) | Internal event emitter |

### Network Interception

| Playwright API | pwright | Status | CDP method |
|---------------|---------|--------|------------|
| `page.route(url, handler)` | — | Planned (Tier 3) | `Fetch.enable` + `Fetch.requestPaused` |
| `page.unroute(url)` | — | Planned (Tier 3) | `Fetch.disable` |
| `page.routeFromHAR(har)` | — | ❌ Out of scope | |
| `page.routeWebSocket(url)` | — | ❌ Out of scope | |
| `page.setExtraHTTPHeaders(h)` | `page.set_extra_http_headers()` | Planned | `Network.setExtraHTTPHeaders` |
| `response.body()` | `session.network_get_response_body()` | ✅ Exists | `Network.getResponseBody` |

### Misc

| Playwright API | pwright | Status | Notes |
|---------------|---------|--------|-------|
| `page.bringToFront()` | `page.bring_to_front()` | ✅ Exists | `Page.bringToFront` |
| `page.mainFrame()` | `page.main_frame()` | Planned | Returns `Frame` |
| `page.frames()` | `page.frames()` | Planned (Tier 3) | All frames incl. iframes |
| `page.frame(name)` | `page.frame()` | Planned (Tier 3) | Find by name/URL |
| `page.workers()` | — | ❌ Out of scope | |
| `page.video()` | — | ❌ Out of scope | |
| `page.coverage` | — | ❌ Out of scope | |
| `page.keyboard` | `page.keyboard()` | ✅ Exists | Returns `Keyboard` |
| `page.mouse` | `page.mouse()` | ✅ Exists | Returns `Mouse` |
| `page.touchscreen` | `page.touchscreen()` | ✅ Exists | Returns `Touchscreen` |

---

## Locator

### Actions

| Playwright API | pwright | Status | Implementation |
|---------------|---------|--------|----------------|
| `locator.click(opts)` | `locator.click()` | ✅ Exists | Resolve → click via box model |
| `locator.dblclick(opts)` | `locator.dblclick()` | ✅ Exists | `clickCount: 2` |
| `locator.fill(value)` | `locator.fill()` | ✅ Exists | Resolve → focus + set value |
| `locator.clear()` | `locator.clear()` | Planned | `fill("")` |
| `locator.type(text)` | `locator.type_text()` | ✅ Exists | Resolve → key events |
| `locator.pressSequentially(t)` | `locator.press_sequentially()` | Planned | Alias for `type` |
| `locator.press(key)` | `locator.press()` | ✅ Exists | Resolve → key dispatch |
| `locator.hover(opts)` | `locator.hover()` | ✅ Exists | Resolve → mouse move |
| `locator.focus()` | `locator.focus()` | ✅ Exists | `DOM.focus` |
| `locator.blur()` | `locator.blur()` | ✅ Exists | JS eval `blur()` |
| `locator.check()` | `locator.check()` | ✅ Exists | Click if unchecked |
| `locator.uncheck()` | `locator.uncheck()` | ✅ Exists | Click if checked |
| `locator.setChecked(bool)` | `locator.set_checked()` | Planned | `check()` or `uncheck()` |
| `locator.selectOption(val)` | `locator.select_option()` | ✅ Exists | Resolve → select JS |
| `locator.tap(opts)` | `page.touchscreen().tap()` | ✅ Exists | `Input.dispatchTouchEvent` |
| `locator.dragTo(target)` | `locator.drag_to()` | Planned (Tier 3) | Mouse event chain |
| `locator.setInputFiles(f)` | `locator.set_input_files()` | ✅ Exists | `DOM.setFileInputFiles` |
| `locator.dispatchEvent(type)` | `locator.dispatch_event()` | ✅ Exists | `Runtime.callFunctionOn` |
| `locator.scrollIntoViewIfNeeded()` | `locator.scroll_into_view()` | ✅ Exists | `DOM.scrollIntoViewIfNeeded` |

### Queries

| Playwright API | pwright | Status | Implementation |
|---------------|---------|--------|----------------|
| `locator.textContent()` | `locator.text_content()` | ✅ Exists | `Runtime.callFunctionOn` |
| `locator.innerText()` | `locator.inner_text()` | ✅ Exists | `Runtime.callFunctionOn` |
| `locator.innerHTML()` | `locator.inner_html()` | ✅ Exists | `DOM.getOuterHTML` |
| `locator.getAttribute(name)` | `locator.get_attribute()` | ✅ Exists | `DOM.getAttributes` (**no JS**) |
| `locator.inputValue()` | `locator.input_value()` | ✅ Exists | `Runtime.callFunctionOn` |
| `locator.isVisible()` | `locator.is_visible()` | ✅ Exists | `DOM.getBoxModel` (**no JS**) |
| `locator.isHidden()` | `locator.is_hidden()` | ✅ Exists | `DOM.getBoxModel` fails (**no JS**) |
| `locator.isEnabled()` | `locator.is_enabled()` | ✅ Exists | AX tree property (**no JS**) |
| `locator.isDisabled()` | `locator.is_disabled()` | ✅ Exists | JS DOM property via `Runtime.callFunctionOn` |
| `locator.isEditable()` | `locator.is_editable()` | Planned | AX tree property (**no JS**) |
| `locator.isChecked()` | `locator.is_checked()` | ✅ Exists | JS DOM property via `Runtime.callFunctionOn` |
| `locator.boundingBox()` | `locator.bounding_box()` | ✅ Exists | `DOM.getBoxModel` |
| `locator.count()` | `locator.count()` | ✅ Exists | `querySelectorAll().length` |

### Composition

| Playwright API | pwright | Status | Notes |
|---------------|---------|--------|-------|
| `locator.first()` | `locator.first()` | ✅ Exists | `querySelectorAll` + index 0 |
| `locator.last()` | `locator.last()` | ✅ Exists | `querySelectorAll` + index -1 |
| `locator.nth(n)` | `locator.nth(n)` | ✅ Exists | `querySelectorAll` + index n |
| `locator.all()` | `locator.all()` | Planned | All matching |
| `locator.filter(opts)` | `locator.filter_by_text()` | ✅ Exists | `hasText` via JS matching |
| `locator.locator(sub)` | `locator.locator()` | ✅ Exists | Scoped sub-query |
| `locator.and(other)` | `locator.and()` | ✅ Exists | CSS `:is()` intersection |
| `locator.or(other)` | `locator.or()` | ✅ Exists | CSS union (`,`) |
| `locator.getByRole/Text/...` | `locator.get_by_*()` | Planned | Scoped getBy queries |

### Waiting & Assertions

| Playwright API | pwright | Status | Implementation |
|---------------|---------|--------|----------------|
| `locator.waitFor(opts)` | `locator.wait_for()` | ✅ Exists | Poll with `WaitState`: Attached, Visible, Hidden, Detached |
| `locator.screenshot(opts)` | `locator.screenshot()` | Planned | Element clip screenshot |
| `locator.ariaSnapshot()` | — | Planned | Per-element accessibility snapshot |
| `locator.evaluate(fn)` | `locator.evaluate()` | ✅ Exists | `Runtime.callFunctionOn` |
| `locator.evaluateAll(fn)` | `locator.evaluate_all()` | Planned | `Runtime.callFunctionOn` |
| `locator.allInnerTexts()` | `locator.all_inner_texts()` | Planned | Eval on all matches |
| `locator.allTextContents()` | `locator.all_text_contents()` | Planned | Eval on all matches |
| `locator.highlight()` | — | ❌ Out of scope | Dev tools only |

---

## Keyboard

| Playwright API | pwright | Status | CDP method |
|---------------|---------|--------|------------|
| `keyboard.down(key)` | `keyboard.down()` | ✅ Exists | `Input.dispatchKeyEvent` rawKeyDown |
| `keyboard.up(key)` | `keyboard.up()` | ✅ Exists | `Input.dispatchKeyEvent` keyUp |
| `keyboard.press(key)` | `keyboard.press()` | ✅ Exists | rawKeyDown + keyUp |
| `keyboard.type(text, opts)` | `keyboard.type_text()` | ✅ Exists | Per-char `Input.insertText` |
| `keyboard.insertText(text)` | `keyboard.insert_text()` | ✅ Exists | `Input.insertText` |

## Mouse

| Playwright API | pwright | Status | CDP method |
|---------------|---------|--------|------------|
| `mouse.click(x, y, opts)` | `mouse.click()` | ✅ Exists | mousePressed + mouseReleased |
| `mouse.dblclick(x, y, opts)` | `mouse.dblclick()` | ✅ Exists | `clickCount: 2` |
| `mouse.move(x, y, opts)` | `mouse.move_to()` | ✅ Exists | mouseMoved |
| `mouse.down(opts)` | `mouse.down()` | ✅ Exists | mousePressed |
| `mouse.up(opts)` | `mouse.up()` | ✅ Exists | mouseReleased |
| `mouse.wheel(dx, dy)` | `mouse.wheel()` | ✅ Exists | mouseWheel |

## Touchscreen

| Playwright API | pwright | Status | CDP method |
|---------------|---------|--------|------------|
| `touchscreen.tap(x, y)` | `touchscreen.tap()` | ✅ Exists | `Input.dispatchTouchEvent` |

---

## Selector Engine

pwright implements a subset of Playwright's selector engine:

| Selector | Syntax | Implementation |
|----------|--------|----------------|
| CSS | `page.locator("button.submit")` | `DOM.querySelector` / `querySelectorAll` |
| Text | `page.get_by_text("Submit")` | JS `textContent` matching via `Runtime.evaluate` |
| Role | `page.get_by_role("button")` | JS implicit role mapping + `[role]` CSS |
| Test ID | `page.get_by_test_id("login")` | CSS `[data-testid="login"]` |
| Label | `page.get_by_label("Email")` | JS `<label>` for-association + `aria-label` |
| Placeholder | `page.get_by_placeholder("...")` | CSS `[placeholder="..."]` |
| Alt text | `page.get_by_alt_text("...")` | CSS `[alt="..."]` |
| Title | `page.get_by_title("...")` | CSS `[title="..."]` |

**Resolution flow:**
```
locator("button.submit").click()
  → DOM.getDocument → root nodeId
    → DOM.querySelector(root, "button.submit") → nodeId
      → DOM.scrollIntoViewIfNeeded(nodeId)
      → DOM.getBoxModel(nodeId) → content quad → (x, y) center
        → Input.dispatchMouseEvent("mousePressed", x, y)
        → Input.dispatchMouseEvent("mouseReleased", x, y)
```

---

## JS Evaluation Strategy

All JavaScript snippets are centralized in the `pwright-js` crate:

| Module | What's there |
|--------|-------------|
| `pwright_js::dom` | `GET_INNER_TEXT`, `query_selector_exists()` |
| `pwright_js::page` | `GET_READY_STATE`, `GET_LOCATION_HREF`, `GET_TITLE`, `GET_DOCUMENT_HTML`, `scroll_by()` |
| `pwright_js::element` | `GET_BOUNDING_CENTER`, `SET_VALUE` |

Where possible, pwright avoids JS and uses pure CDP domains:
- **DOM queries**: `DOM.getAttributes`, `DOM.getOuterHTML`, `DOM.getBoxModel`
- **Visibility checks**: `DOM.getBoxModel` success/failure (`is_visible`, `is_hidden`)
- **Input**: `Input.dispatchKeyEvent`, `Input.dispatchMouseEvent`

JS evaluation (`Runtime.callFunctionOn`) is used for:
- `innerText`, `value` property, `scrollBy`
- `is_checked()`, `is_disabled()` (JS DOM properties are more accurate than HTML attributes)
- Text/label/role selector resolution
- Per-element `locator.evaluate()`

---

## Permanently Out of Scope

| Feature | Why |
|---------|-----|
| `BrowserType.launch()` | Remote CDP only |
| `BrowserType.launchServer()` | Remote CDP only |
| `Tracing` | Requires server-side file I/O |
| `Video` / `Screencast` | Requires ffmpeg/media pipeline |
| `Electron` / `Android` | Different platforms |
| `Clock` (fake timers) | Test utility, not bridge concern |
| `Coverage` | Chrome profiler domains, niche |
| `HAR` recording/replay | Requires filesystem |
| Test runner / `expect` | Not a testing library |
| `Selectors.register()` | Custom selector engines |
| `APIRequestContext` | HTTP client, not browser automation |

---

## Implementation Phases

| Phase | Scope | Status |
|-------|-------|--------|
| **1** | `Page` + `Locator` (CSS) + `Keyboard` + `Mouse` | ✅ Complete |
| **2** | `getBy*` queries + file upload + download | ✅ Complete |
| **3** | `Touchscreen` + locator composition (`and`, `or`, `filter`) | ✅ Complete |
| **4** | `Frame` (iframes) + `BrowserContext` + request interception | Planned |
