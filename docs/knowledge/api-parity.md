# API Parity: gRPC vs CLI vs Playwright Bridge

This document tracks feature coverage across pwright's three API surfaces.

## Coverage Matrix

| Feature | Rust (Playwright API) | gRPC | CLI | Notes |
|---------|:---------------------:|:----:|:---:|-------|
| **Navigation** |
| Navigate to URL | `page.goto()` | `Navigate` | `open`, `goto` | ✅ All |
| Reload | `page.reload()` | `Reload` | `reload` | ✅ All |
| Go back | `page.go_back()` | `GoBack` | `go-back` | ✅ All |
| Go forward | `page.go_forward()` | `GoForward` | `go-forward` | ✅ All |
| **Content** |
| Accessibility snapshot | bridge `get_snapshot()` | `GetSnapshot` | `snapshot` | ✅ gRPC + CLI |
| Screenshot | `page.screenshot()` | `TakeScreenshot` | `screenshot` | ✅ All |
| PDF | `page.pdf()` | `GetPDF` | `pdf` | ✅ All |
| Text extraction | `page.body_text()` | `GetText` | via `eval` | ✅ All |
| Page title | `page.title()` | via `Evaluate` | via `eval` | ✅ All |
| Page content | `page.content()` | via `Evaluate` | via `eval` | ✅ All |
| JS evaluate | `page.evaluate()` | `Evaluate` | `eval` | ✅ All |
| **Actions** |
| Click | `locator.click()` | `ExecuteAction(CLICK)` | `click` | ✅ All |
| Type text | `locator.type_text()` | `ExecuteAction(TYPE)` | `type` | ✅ All |
| Fill | `locator.fill()` | `ExecuteAction(FILL)` | `fill` | ✅ All |
| Press key | `keyboard.press()` | `ExecuteAction(PRESS)` | `press` | ✅ All |
| Hover | `locator.hover()` | `ExecuteAction(HOVER)` | `hover` | ✅ All |
| Select | `locator.select_option()` | `ExecuteAction(SELECT)` | `select` | ✅ All |
| Scroll | `locator.scroll_into_view()` | `ExecuteAction(SCROLL)` | via `eval` | ✅ All |
| Drag | `locator.drag()` | `ExecuteAction(DRAG)` | `drag` | ✅ All |
| Focus | `locator.focus()` | `ExecuteAction(FOCUS)` | `focus` | ✅ All |
| Check | `locator.check()` | `ExecuteAction(CHECK)` | via `click` | ✅ All |
| Uncheck | `locator.uncheck()` | `ExecuteAction(UNCHECK)` | via `click` | ✅ All |
| Double-click | `locator.dblclick()` | `ExecuteAction(DBLCLICK)` | via `eval` | ✅ gRPC + Rust |
| Blur | `locator.blur()` | ❌ | ❌ | Rust API only |
| Dispatch event | `locator.dispatch_event()` | ❌ | ❌ | Rust API only |
| **File Operations** |
| File upload | `locator.set_input_files()` | `SetInputFiles` | `upload` | ✅ All |
| File download | `content::expect_download()` | `ExpectDownload` | `download` | ✅ All |
| **Touchscreen** |
| Tap | `touchscreen.tap()` | `TouchTap` | ❌ | ✅ gRPC + Rust |
| **Tab Management** |
| List tabs | via `CdpSession` | `ListTabs` | `tab-list` | ✅ gRPC + CLI |
| Create tab | via `CdpSession` | `CreateTab` | `tab-new` | ✅ gRPC + CLI |
| Close tab | `Page::close()` | `CloseTab` | `tab-close` | ✅ All |
| Bring to front | `page.bring_to_front()` | `BringToFront` | `tab-select` | ✅ All |
| **Cookies** |
| Get cookies | `browser.get_cookies()` | `GetCookies` | `cookie-list` | ✅ All |
| Set cookies | `browser.set_cookies()` | `SetCookies` | `cookie-set` | ✅ All |
| **Health** |
| Health check | — | `Health` | `health` | ✅ |

## Remaining Gaps

Only 2 minor features are **not** available across all surfaces:

| Feature | Available in | Missing from | Rationale |
|---------|-------------|-------------|-----------|
| Blur | Rust API | gRPC, CLI | Rarely needed; use `evaluate('el.blur()')` |
| Dispatch event | Rust API | gRPC, CLI | Generic — `Evaluate` covers all cases |

### Locator APIs (Rust-only, intentionally)

The `getBy*` locator APIs (`get_by_text`, `get_by_label`, `get_by_role`, `filter_by_text`, `and`/`or`) are **Rust-only by design**. They require complex selector resolution that doesn't map cleanly to a request/response protocol. The gRPC `ExecuteAction` with CSS selectors and `Evaluate` covers the common cases.

### New Rust-only APIs (post-improvement)

| Feature | Available in | Notes |
|---------|-------------|-------|
| `Browser::connect_http()` | Rust API | CDP URL discovery via `/json/version` |
| `Browser::new_tab()` / `TabHandle::close()` | Rust API | Explicit tab lifecycle management |
| `Page::close()` | Rust API | Close tab via `Target.closeTarget` |
| `Page::on_response()` | Rust API | Network response event channel |
| `Page::on_request()` | Rust API | Network request event channel |
| `Locator::evaluate()` | Rust API | Per-element JS evaluation |
| `Locator::wait_for(ms, state)` | Rust API | Wait with visibility states |
| `Page::evaluate_async()` | Rust API | JS eval with `awaitPromise: true` for Promises |
| `CdpSession::network_get_response_body()` | Rust API | Get response body by request ID |
| `Locator::nth(n)` | Rust API | Index into query results |
| `Page::wait_for_response()` | Rust API | Wait for network response matching predicate |
| `Page::wait_for_request()` | Rust API | Wait for network request matching predicate |
| `Page::response_body()` | Rust API | Get response body by request ID |
| `Page::evaluate_with_arg()` | Rust API | Pass args via CDP serialization (no JS injection) |
