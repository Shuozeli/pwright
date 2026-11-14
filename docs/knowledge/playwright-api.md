<!-- agent-updated: 2026-10-06T21:07:32Z -->

# Playwright API Compatibility

pwright aims to provide a Rust API with the same mental model as [Playwright](https://playwright.dev). This document maps every Playwright public API to its pwright status: **supported**, **planned**, or **out of scope**.

> **Constraint:** pwright only supports **remote CDP** — connecting to an externally-managed Chrome via WebSocket. No browser launch, no profiles, no tracing, no video.

Node NODE-427 adds opt-in `Browser.newContext({ hasTouch: true })` over the
existing Rust device-emulation registry and completes native TouchEvent
initializer materialization for `Locator.dispatchEvent()`. The unchanged
Chromium touch-dispatch file passes 1/1 candidate and oracle. Other nondefault
device options and uncomposed Context-policy combinations remain explicit
future boundaries.

Python ABI 96 now covers the complete async/sync Chromium request-fallback
files: source candidate and official Playwright 1.58.0 oracle pass 55/55.
`Request.headers` exposes lowercase names, `all_headers()` returns a copy and
`header_value()` is case-insensitive. Rust continues to own effective fallback
matching and settlement; Python retains the published effective Request
identity in a 64-entry bound through terminal network publication. Sync route
handlers may call sync Page APIs without blocking their own bridge loop. Final
installed-wheel browser promotion is pending until the shared CDP endpoint is
available again.

Python ABI 96 adds `BrowserContext.request`, `Page.request`, `Route.fetch()`
and `Route.fulfill(json=...)` through two opt-in Rust compositions. The Context
owner supplies live cookies and bounded base-URL/user-agent/credential/TLS
settings to the existing API-request client; the route owner replays exact
paused-request identity and can fulfill from its retained `APIResponse`.
Python keeps public callback/value identity only. Final installed candidate
and official Playwright 1.58.0 oracle pass all 108 selected expanded Chromium
rows (51 + 47 + 6 + 4).

Python ABI 87 completes Page `request`, `requestfinished` and `requestfailed`
listeners. Rust owns the three native subscriptions, bounded notice FIFO,
failure text and exact native-to-portable identity correlation. Python owns
only one watcher and public callback dispatch. The unchanged abort rows pass
candidate/oracle 2/2 and repeat 10/10.

Python ABI 86 adds async/sync `set_extra_http_headers()` on Page and
BrowserContext. Rust owns bounded settings, Page-over-Context precedence,
native session fanout and implicit Page-referrer composition. CDP request
header names are lowercased before route publication, matching Playwright's
public mapping. The three unchanged official Chromium rows pass installed
candidate/oracle 3/3 and candidate repeat 15/15. Two additional unchanged
Page-route rows for native `Referer`/`Origin` lowercase publication pass
candidate/oracle 2/2 and repeat 10/10 without another ABI change.

Python ABI 86 also keeps route-fulfill status semantics in Rust. Every
Chromium `Fetch.fulfillRequest` carries Playwright's pinned reason phrase,
including `422 Unprocessable Entity`, rather than inheriting browser-version
defaults. The unchanged 422 row passes candidate/oracle 1/1 and repeat 5/5.

Python ABI 85 adds async/sync handler-specific `unroute()` on Page and
BrowserContext plus composable `Route.fallback()` options. Rust owns the LIFO
Page-before-Context traversal, effective-request rematching, field-wise
override merge and terminal settlement. The eight unchanged official
Chromium rows pass installed candidate/oracle 8/8 and repeat 40/40.

Python ABI 84 adds finite positive `times` budgets to async/sync Page and
BrowserContext bounded string routes. Rust atomically matches, decrements and
removes the registration, refreshes affected Fetch patterns after the final
terminal action and continues transient stale-pattern pauses without calling
Python. `times=0` remains unlimited. The three unchanged official Chromium
rows pass installed candidate/oracle 3/3 and candidate repeat 15/15.

Python ABI 83 route globs now match Playwright's question-mark rule: `?` is a
literal, not a one-character wildcard. Rust applies that rule during final
selection and compiles the same bounded escaped pattern for Chromium's Fetch
prefilter. The unchanged official row passes installed candidate/oracle 1/1
and candidate repeat 5/5.

Python ABI 83 adds complete async/sync `Route.continue_()` override values for
the currently supported Chromium routing slice: URL, method, headers and POST
data. Rust validates bounds and same-protocol URL replacement, owns the paused
request identity and emits one `Fetch.continueRequest`; Python only normalizes
public values. The ten unchanged request-continue rows pass installed
candidate and independent oracle 10/10. Regex/predicate matchers,
Regex/predicate matching and broader interception semantics remain planned.

Python ABI 82 adds opt-in async/sync `Page.unroute_all()` and
`BrowserContext.unroute_all()` with Playwright's default and `"wait"`
behaviors. Rust removes exactly the selected Page or Context scope, refreshes
Fetch across affected current sessions and, for `"wait"`, waits until both
pending and callback-settling calls from that scope retire. Python clears its
callback identities only after native removal completes. `"ignoreErrors"`
remains explicitly unsupported. Six unchanged default-removal and in-flight-
wait rows pass installed candidate and independent oracle 6/6.

Python ABI 81 adds opt-in async/sync `BrowserContext.route()` for bounded
string glob/path patterns. Rust owns Context scope, base-URL resolution,
Page-over-Context precedence, current/future Page/OOPIF initialization and all
Fetch state; Python retains only the callback keyed by an opaque registration.
The four unchanged Context HAR URL-filter composition rows pass installed
candidate/oracle 4/4 and candidate stress async 20/20 plus sync 20/20.

Python ABI 80 also treats a present HAR `log` without `entries` as an empty
archive. `not_found="fallback"` consequently continues the request; invalid
JSON, malformed entries, external content and bound violations still fail.

Python ABI 80 adds opt-in `BrowserContext.route_from_har()` over ABI 79's
bounded plain embedded-HAR owner. Rust owns Page/Context scope, Page precedence,
future-Page/OOPIF installation and a fixed 64-session initialization bridge.
The six new unchanged rows pass installed candidate/oracle 6/6 and candidate
stress async 30/30 plus sync 30/30. External content, ZIP/update archives,
regex/predicate filters, handler-specific `unroute()` and
`unroute_all(behavior="ignoreErrors")` remain planned.

Python ABI 79 adds opt-in `Page.route_from_har()` plus Rust-owned computed-
style polling for `expect(locator).to_have_css()`. The persistent-Context,
async/sync Page method/redirect and async/sync Page URL-filter/ordinary-route-
composition rows pass installed candidate/oracle 5/5.

Python ABI 78 adds opt-in ordinary/persistent Context `http_credentials` and
navigation `Response.body()`. Rust owns bounded credentials, origin matching,
Fetch challenge settlement, request/session routing and response bytes; Python
only adapts public values.

Python ABI 19 adds async/sync `Page.drag_and_drop` and `Locator.drag_to` for
the fixed Chromium center/position/step slice. Rust owns same-Page/same-Frame
validation, locator geometry and bounded Mouse/HTML5 drag input. Full
actionability/hit-target retry semantics, force mode and other browsers remain
planned. Python ABI 18 additionally supports async/sync `Request.sizes()` through a
bounded opaque Rust owner with Chromium raw-header correlation. Python ABI 17
supports async Page `route` for bounded string
glob/path patterns with `continue_()` and `abort()`. Rust owns interception,
selection, pending-call capacity and terminal Page-close handling. Regex and
predicate matchers, `times`, overrides, fulfill/fallback/unroute, Context and
sync routes remain planned. ABI 16 supports Page `pageerror` with
Chromium-compatible Error name, message, stack and thrown-value conversion.
ABI 15 supports Page `load`, `domcontentloaded`, `close`
and `popup` events, `opener`, event waiters/listeners, and the ABI 14 Page
`expose_function` / `expose_binding` slice. Event observation, publication and
close retirement remain Rust-owned; Python owns public wrapper/callback
identity only.

Python ABI 13 introduced a bounded first Handle slice:
`Page`/`Frame.evaluate_handle`, direct Handle arguments, `json_value`,
`as_element`, `wait_for_function`, `query_selector`, and Page script/style
tags. Rust owns remote-object identity, capacity, polling and disposal. Nested
Handle graphs, properties and most ElementHandle methods remain planned.

---

## Object Model

```
Playwright
  ├─ request.newContext()                 ← browser-independent
  └─ BrowserType.connectOverCDP(wsEndpoint)
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
| `APIRequestContext` | `pwright_api_request::ApiRequestContext` | ✅ Node/Python opt-in |
| `APIResponse` | `pwright_api_request::ApiResponse` | ✅ Node/Python opt-in |

---

## Browser

| Playwright API | pwright | Status | Notes |
|---------------|---------|--------|-------|
| `browser.newContext(opts)` | `browser.new_context()` | Partial | Node opt-in Context creation supports selected base URL, headers, proxy/CSP and `hasTouch`; other nondefault device/policy options remain explicit |
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
| `context.newPage()` | `context.new_page()` | ✅ Python | Rust-owned `Target.createTarget` lifecycle |
| `context.pages()` | `context.pages()` | ✅ Python | Published Rust topology |
| `context.cookies(urls)` | `context.cookies()` | ✅ Exists | `Network.getCookies` |
| `context.addCookies(cookies)` | `context.add_cookies()` | ✅ Exists | `Network.setCookie` |
| `context.clearCookies()` | `context.clear_cookies()` | Planned | `Network.clearBrowserCookies` |
| `context.close()` | `context.close()` | ✅ Python | Rust child-first lifecycle |
| `context.setDefaultTimeout(ms)` | `context.set_default_timeout()` | ✅ Python ABI 8 opt-in | Fixed Rust timeout table |
| `context.setDefaultNavigationTimeout(ms)` | `context.set_default_navigation_timeout()` | ✅ Python ABI 8 opt-in | Fixed Rust timeout table |
| `context.grantPermissions(perms)` | `context.grant_permissions()` | Planned | `Browser.grantPermissions` |
| `context.clearPermissions()` | `context.clear_permissions()` | Planned | `Browser.resetPermissions` |
| `context.setGeolocation(geo)` | `context.set_geolocation()` | Planned | `Emulation.setGeolocationOverride` |
| `context.setExtraHTTPHeaders(h)` | `context.set_extra_http_headers()` | ✅ Python ABI 86 opt-in | Bounded Rust settings + `Network.setExtraHTTPHeaders` fanout |
| `context.setOffline(bool)` | `context.set_offline()` | Planned | `Network.emulateNetworkConditions` |
| new/persistent Context `http_credentials` option | Rust Context policy | ✅ Python ABI 78 opt-in | Selected-session `Fetch.enable` + exact challenge settlement |
| `context.addInitScript(script)` | `context.add_init_script()` | ✅ Node/Python opt-in | Bounded Rust scope + `Page.addScriptToEvaluateOnNewDocument` |
| `context.routeFromHAR(har)` | `context.route_from_har()` | Partial: Python ABI 80 opt-in | Plain embedded HAR, bounded Rust Context scope and Page/OOPIF Fetch settlement |
| `context.storageState()` | — | ❌ Out of scope | Requires local filesystem |
| `context.route(url, handler)` | `context.route()` | Partial: Python ABI 81 opt-in | Bounded string patterns; Rust Context scope and Fetch settlement |
| `context.unroute(url)` | — | Planned (Tier 3) | `Fetch.disable` |
| `context.unrouteAll(opts)` | `context.unroute_all()` | Partial: Python ABI 82 opt-in | Default and `"wait"`; exact Context scope; `"ignoreErrors"` unsupported |
| `context.exposeBinding()` | — | Planned (Tier 3) | `Runtime.addBinding` |
| `context.exposeFunction()` | — | Planned (Tier 3) | `Runtime.addBinding` |
| `context.tracing` | — | ❌ Out of scope | |
| `context.request` (APIRequestContext) | `pwright_api_request::ApiRequestContext` | Planned | Global `playwright.request` is public; BrowserContext cookie sharing is pending |

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
| `page.addInitScript(s)` | `page.add_init_script()` | ✅ Node/Python opt-in | Bounded Rust scope + current/future target-session installation |
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
| `page.selectOption(selector, v)` | `page.select_option()` | ✅ Exists | Legacy value-only bridge; v2 Locator route is the semantic implementation |
| `page.setInputFiles(sel, f)` | `page.set_input_files()` | ✅ Exists | `DOM.setFileInputFiles` |
| `page.tap(selector)` | `page.touchscreen().tap()` | ✅ Exists | `Input.dispatchTouchEvent` |
| `page.dragAndDrop(s, t)` | `page.drag_and_drop()` | Partial: Python ABI 19 | Rust-owned bounded locator/Mouse drag; force and complete actionability remain open |

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
| `page.emulateMedia(opts)` | `page.emulate_media()` | ✅ Python ABI 7 opt-in | Rust-owned bounded override state + `Emulation.setEmulatedMedia` |

### Waiting

| Playwright API | pwright | Status | Implementation |
|---------------|---------|--------|----------------|
| `page.waitForTimeout(ms)` | `page.wait_for_timeout()` | ✅ Exists | `tokio::time::sleep` |
| `page.waitForFunction(fn)` | `page.wait_for_function()` | Planned (Tier 3) | Poll `Runtime.evaluate` |
| `page.waitForSelector(sel)` | `page.wait_for_selector()` | ✅ Exists | Poll `DOM.querySelector` |
| `page.waitForRequest(url)` | `page.wait_for_request()` | ✅ Python ABI 9 opt-in | Bounded Rust subscription/waiter over normalized network events |
| `page.waitForResponse(url)` | `page.wait_for_response()` | ✅ Python ABI 9 opt-in | Bounded Rust subscription/waiter over normalized network events |
| `page.waitForEvent(ev)` | `page.wait_for_event()` | Partial: network events | ABI 9 request/response/finished/failed only |

### Network Interception

| Playwright API | pwright | Status | CDP method |
|---------------|---------|--------|------------|
| `page.route(url, handler)` | `page.route()` | Partial: Python ABI 86 opt-in | Bounded string patterns and finite `times`; continue/fallback URL/method/header/POST-data overrides, abort and fulfill with Playwright reason phrases; regex/predicate gaps |
| `page.unroute(url, handler=None)` | `page.unroute()` | Partial: Python ABI 85 opt-in | Exact bounded string-pattern removal; optional original handler identity; regex/predicate gaps |
| `page.unrouteAll(opts)` | `page.unroute_all()` | Partial: Python ABI 82 opt-in | Default and `"wait"`; exact Page scope; `"ignoreErrors"` unsupported |
| `page.routeFromHAR(har)` | `page.route_from_har()` | Partial: Python ABI 79 opt-in | Plain embedded HAR, bounded Rust matching and Page Fetch settlement |
| `page.routeWebSocket(url)` | — | ❌ Out of scope | |
| `page.setExtraHTTPHeaders(h)` | `page.set_extra_http_headers()` | ✅ Python ABI 86 opt-in | Bounded Rust Page override, navigation referer and route snapshot composition |
| `response.body()` | `session.network_get_response_body()` | ✅ Node/Python opt-in | Bounded Rust-owned `Network.getResponseBody` settlement |
| `request.sizes()` | `request.sizes()` | ✅ Python ABI 18 opt-in | Rust-owned terminal size settlement + raw request headers |

### Misc

| Playwright API | pwright | Status | Notes |
|---------------|---------|--------|-------|
| `page.bringToFront()` | `page.bring_to_front()` | ✅ Exists | `Page.bringToFront` |
| `page.mainFrame()` | `page.main_frame()` | ✅ Python | Published Rust Frame tree |
| `page.frames()` | `page.frames()` | ✅ Python | Published Rust Frame tree |
| `page.frame(name)` | `page.frame()` | ✅ Python | Name/URL/regex/callable facade lookup |
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
| `locator.clear()` | `locator.clear()` | ✅ Exists | Client composition over `fill("")` |
| `locator.type(text)` | `locator.type_text()` | ✅ Exists | Resolve → key events |
| `locator.pressSequentially(t)` | `locator.press_sequentially()` | Planned | Alias for `type` |
| `locator.press(key)` | `locator.press()` | ✅ Exists | Resolve → key dispatch |
| `locator.hover(opts)` | `locator.hover()` | ✅ Exists | Resolve → mouse move |
| `locator.focus()` | `locator.focus()` | ✅ Exists | `DOM.focus` |
| `locator.blur()` | `locator.blur()` | ✅ Exists | JS eval `blur()` |
| `locator.check()` | `locator.check()` | ✅ Exists | Click if unchecked |
| `locator.uncheck()` | `locator.uncheck()` | ✅ Exists | Click if checked |
| `locator.setChecked(bool)` | `locator.set_checked()` | ✅ Exists | Desired-state primitive; check/uncheck compose over it |
| `locator.selectOption(val)` | `locator.select_option()` | Partial | Locator 1.21/2.4 strings and descriptors; ElementHandle overload pending explicit adoption design |
| `locator.tap(opts)` | `page.touchscreen().tap()` | ✅ Exists | `Input.dispatchTouchEvent` |
| `locator.dragTo(target)` | `locator.drag_to()` | Partial: Python ABI 19 | Same selected drag leaf as Page; full actionability/force remain open |
| `locator.setInputFiles(f)` | `locator.set_input_files()` | ✅ Exists | `DOM.setFileInputFiles` |
| `locator.dispatchEvent(type)` | `locator.dispatch_event()` | ✅ Exists | `Runtime.callFunctionOn` |
| `locator.scrollIntoViewIfNeeded()` | `locator.scroll_into_view_if_needed()` | Locator 1.22/2.5 | Strict lazy Locator route; stable-only wait and full-intersection no-op, implemented in the Chromium utility world |

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

## APIRequest

The optional Node and Python profiles expose browser-independent
`playwright.request.newContext()`. Rust owns request construction, DNS/socket/TLS,
redirects, retries, Basic authentication, cookies, storage state, decompression,
timeouts, cancellation and disposal. Language facades only validate and convert
public values. The separately selectable Node and Python client-certificate
profiles add
bounded exact-origin PEM/PFX mutual TLS with lazy Rust decoding and cached
Rustls configurations. The official validation row passes 3/3 and seven direct
request behaviors pass 21/21 in both candidate and oracle lanes. BrowserContext
cookie/certificate sharing, proxy paths and multiple identities for one origin
are still planned.

| Playwright API | pwright | Status |
|---------------|---------|--------|
| `request.newContext(opts)` | `ApiRequestContext::new()` | ✅ Node/Python opt-in |
| `context.fetch(url, opts)` | `ApiRequestContext::fetch()` | ✅ Node/Python opt-in |
| `context.get/post/put/patch/delete/head(url, opts)` | Rust request pipeline | ✅ Node/Python opt-in |
| `context.storageState(opts)` | Rust cookies/origins plus facade path I/O | ✅ Node/Python opt-in |
| `context.dispose(opts)` | Rust cancellation and response reclamation | ✅ Node/Python opt-in |
| `response.body/text/json()` | bounded Rust-owned response body | ✅ Node/Python opt-in |
| `response.headers()/headersArray()` | preserved final response headers | ✅ Node/Python opt-in |
| `context.request` | shared BrowserContext cookie store | Planned |
| client-certificate input validation | thin Node/Python shape validators | ✅ Node/Python |
| direct exact-origin PEM/PFX client certificate | optional Rust identity provider | ✅ Node/Python opt-in |
| proxy / browser-context certificate transport | separate optional adapters | Planned |
| multiple client identities for one origin | Playwright-compatible selection | Planned |

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
| Test runner / `expect` | Not a testing library |
| `Selectors.register()` | Custom selector engines |

---

## Implementation Phases

| Phase | Scope | Status |
|-------|-------|--------|
| **1** | `Page` + `Locator` (CSS) + `Keyboard` + `Mouse` | ✅ Complete |
| **2** | `getBy*` queries + file upload + download | ✅ Complete |
| **3** | `Touchscreen` + locator composition (`and`, `or`, `filter`) | ✅ Complete |
| **4** | `Frame` (iframes) + `BrowserContext` + request interception | Planned |
