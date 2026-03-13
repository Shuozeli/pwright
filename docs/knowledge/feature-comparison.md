# Feature Comparison: Playwright vs PinchTab vs pwright

> pwright is a **Playwright-compatible** Rust CDP bridge. It covers the core browser control surface and exposes it as both a Rust library (with a Playwright-style Page/Locator API), a gRPC service, and a CLI. PinchTab is a full browser control plane with orchestration, profiles, security, and a dashboard. Playwright is the original, full-featured Microsoft framework handling everything from browser installation to high-level test assertions.

## At a Glance

| Dimension | Playwright | PinchTab | pwright |
|-----------|------------|----------|---------|
| Language | TS/JS, Python, Java, .NET | Go | Rust |
| Binary size | Large (Node/Py + Browsers) | ~12 MB | ~5.9 MB |
| API transport | Node IPC / WebSocket | HTTP/REST | gRPC + Rust lib + CLI |
| Architecture | Node server + multiple client bindings | All-in-one control plane | Thin bridge + Playwright API |
| CDP library | Custom | chromedp (Go) | Raw WebSocket (custom) |
| Process model | Framework manages browser execution | Server manages Chrome lifecycle | Attach to existing Chrome |
| API style | Playwright | Custom REST endpoints | Playwright-compatible Page/Locator |

---

## Browser Actions

| Action | Playwright | PinchTab | pwright | Notes |
|--------|:----------:|:--------:|:-------:|-------|
| Click (by selector) | ✅ | ✅ | ✅ | `locator("button").click()` |
| Click (by ref) | ❌ | ✅ | ✅ | gRPC `ExecuteAction` |
| Double-click | ✅ | ✅ | ✅ | `mouse.dblclick()` |
| Type text | ✅ | ✅ | ✅ | `locator.type_text()` |
| Fill value | ✅ | ✅ | ✅ | `locator.fill()` |
| Press key | ✅ | ✅ | ✅ | `keyboard.press()` / `locator.press()` |
| Focus | ✅ | ✅ | ✅ | `locator.focus()` |
| Blur | ✅ | ❌ | ✅ | `locator.blur()` |
| Hover | ✅ | ✅ | ✅ | `locator.hover()` |
| Check/Uncheck | ✅ | ❌ | ✅ | `locator.check()` / `locator.uncheck()` |
| Select (`<select>`) | ✅ | ✅ | ✅ | `locator.select_option()` |
| Scroll (element) | ✅ | ✅ | ✅ | `locator.scroll_into_view()` |
| Scroll (page) | ✅ | ✅ | ✅ | `mouse.wheel()` |
| Drag | ✅ | ✅ | ✅ | `drag_by_node_id()` |
| Dispatch event | ✅ | ❌ | ✅ | `locator.dispatch_event()` |
| Human-like click | ❌ | ✅ | ❌ | Bezier curves, random delays |
| Human-like type | ❌ | ✅ | ❌ | Variable keystroke timing |

---

## Navigation

| Feature | Playwright | PinchTab | pwright | Notes |
|---------|:----------:|:--------:|:-------:|-------|
| Navigate to URL | ✅ | ✅ | ✅ | `page.goto()` |
| Reload | ✅ | ❌ | ✅ | `page.reload()` |
| Go back/forward | ✅ | ❌ | ✅ | `page.go_back()` / `page.go_forward()` |
| Wait: DOM ready | ✅ | ✅ | ✅ | |
| Wait: network idle | ✅ | ✅ | ✅ | |
| Wait: CSS selector | ✅ | ✅ | ✅ | `locator.wait_for()` |
| Wait: timeout | ✅ | ❌ | ✅ | `page.wait_for_timeout()` |
| Block images | ✅ | ✅ | ✅ | |
| Block media | ✅ | ✅ | ✅ | |
| Configurable timeout | ✅ | ✅ | ✅ | |

---

## Content Extraction

| Feature | Playwright | PinchTab | pwright | Notes |
|---------|:----------:|:--------:|:-------:|-------|
| Accessibility snapshot | ✅ | ✅ | ✅ | |
| Screenshot (PNG/JPEG) | ✅ | ✅ | ✅ | `page.screenshot()` |
| Full-page screenshot | ✅ | ✅ | ✅ | |
| PDF export | ✅ | ✅ | ✅ | `page.pdf()` |
| Text extraction | ✅ | ✅ | ✅ | `page.text_content()` |
| Page title | ✅ | ❌ | ✅ | `page.title()` |
| Page content (HTML) | ✅ | ❌ | ✅ | `page.content()` |
| Page URL | ✅ | ❌ | ✅ | `page.url()` |
| JS evaluate | ✅ | ✅ | ✅ | `page.evaluate()` |
| Cookies get/set | ✅ | ✅ | ✅ | |
| Element text content | ✅ | ❌ | ✅ | `locator.text_content()` |
| Element inner text | ✅ | ❌ | ✅ | `locator.inner_text()` |
| Element inner HTML | ✅ | ❌ | ✅ | `locator.inner_html()` |
| Element attributes | ✅ | ❌ | ✅ | `locator.get_attribute()` |
| Input value | ✅ | ❌ | ✅ | `locator.input_value()` |
| Bounding box | ✅ | ❌ | ✅ | `locator.bounding_box()` |
| Element visibility | ✅ | ❌ | ✅ | `locator.is_visible()` / `is_hidden()` |
| Element state | ✅ | ❌ | ✅ | `is_enabled()`, `is_disabled()`, `is_checked()` |
| Element count | ✅ | ❌ | ✅ | `locator.count()` |
| File download | ✅ | ✅ | ✅ | `page.expect_download()` / CLI `download` |
| File upload | ✅ | ✅ | ✅ | `locator.set_input_files()` / `DOM.setFileInputFiles` |

---

## Locator & Selector Engine

| Feature | Playwright | PinchTab | pwright | Notes |
|---------|:----------:|:--------:|:-------:|-------|
| CSS selector | ✅ | ✅ | ✅ | `page.locator()` |
| By test ID | ✅ | ❌ | ✅ | `page.get_by_test_id()` |
| By placeholder | ✅ | ❌ | ✅ | `page.get_by_placeholder()` |
| By alt text | ✅ | ❌ | ✅ | `page.get_by_alt_text()` |
| By title | ✅ | ❌ | ✅ | `page.get_by_title()` |
| Scoped sub-locator | ✅ | ❌ | ✅ | `locator.locator()` |
| First/Last | ✅ | ❌ | ✅ | `locator.first()` / `locator.last()` |
| By role | ✅ | ❌ | ✅ | `page.get_by_role()` with implicit role mapping |
| By text | ✅ | ❌ | ✅ | `page.get_by_text()` via JS text matching |
| By label | ✅ | ❌ | ✅ | `page.get_by_label()` via label/aria lookup |
| Filter (hasText) | ✅ | ❌ | ✅ | `locator.filter_by_text()` |
| And/Or composition | ✅ | ❌ | ✅ | `locator.and()` / `locator.or()` |

---

## Input Devices

| Feature | Playwright | PinchTab | pwright | Notes |
|---------|:----------:|:--------:|:-------:|-------|
| Keyboard press | ✅ | ✅ | ✅ | `keyboard.press()` |
| Keyboard down/up | ✅ | ❌ | ✅ | `keyboard.down()` / `keyboard.up()` |
| Keyboard type | ✅ | ✅ | ✅ | `keyboard.type_text()` |
| Keyboard insert | ✅ | ❌ | ✅ | `keyboard.insert_text()` |
| Mouse click | ✅ | ✅ | ✅ | `mouse.click()` |
| Mouse dblclick | ✅ | ❌ | ✅ | `mouse.dblclick()` |
| Mouse move | ✅ | ❌ | ✅ | `mouse.move_to()` |
| Mouse down/up | ✅ | ❌ | ✅ | `mouse.down()` / `mouse.up()` |
| Mouse wheel | ✅ | ❌ | ✅ | `mouse.wheel()` |
| Touchscreen | ✅ | ❌ | ✅ | `page.touchscreen().tap()` |

---

## Tab Management

| Feature | Playwright | PinchTab | pwright | Notes |
|---------|:----------:|:--------:|:-------:|-------|
| Create tab (page) | ✅ | ✅ | ✅ | |
| Close tab | ✅ | ✅ | ✅ | |
| List tabs | ✅ | ✅ | ✅ | |
| Bring to front | ✅ | ❌ | ✅ | `page.bring_to_front()` |
| Tab locking (mutex) | ❌ | ✅ | ❌ | Owner-based TTL locks |
| Tab executor (parallel) | ✅ | ✅ | ✅ | Per-tab Mutex + cross-tab Semaphore |
| Tab screencast (live) | ❌ | ✅ | ❌ | WebSocket-based live video |

---

## CLI

| Feature | Playwright | PinchTab | pwright | Notes |
|---------|:----------:|:--------:|:-------:|-------|
| CLI tool | ✅ | ✅ | ✅ | `playwright-cli` / `pwright-cli` |
| Navigate command | ✅ | ✅ | ✅ | |
| Snapshot command | ✅ | ✅ | ✅ | |
| Screenshot command | ✅ | ✅ | ✅ | |
| Evaluate command | ✅ | ✅ | ✅ | |
| Click command | ✅ | ✅ | ✅ | |
| Type command | ❌ | ✅ | ✅ | |
| Fill command | ❌ | ✅ | ✅ | |
| Download command | ❌ | ❌ | ✅ | |

---

## Instance & Profile Management

| Feature | Playwright | PinchTab | pwright | Notes |
|---------|:----------:|:--------:|:-------:|-------|
| Browser orchestrator | partial | ✅ | ❌ | Playwright test worker scaling |
| Profile persistence | ✅ | ✅ | ❌ | UserDataDir context |
| Profile CRUD | ❌ | ✅ | ❌ | |
| Browser lifecycle mgmt | ✅ | ✅ | ❌ | |
| Allocation policies | ❌ | ✅ | ❌ | |

---

## Security & Stealth

| Feature | Playwright | PinchTab | pwright | Notes |
|---------|:----------:|:--------:|:-------:|-------|
| Stealth mode | plugins | ✅ | ❌ | Playwright requires playwright-stealth |
| IDPI (prompt defense) | ❌ | ✅ | ❌ | |
| API token auth | ❌ | ✅ | ❌ | |
| Ad blocking | ❌ | ✅ | ❌ | |

---

## Infrastructure

| Feature | Playwright | PinchTab | pwright | Notes |
|---------|:----------:|:--------:|:-------:|-------|
| Web dashboard | ✅ | ✅ | ❌ | Playwright UI Mode / HTML Report |
| OpenAPI spec | ❌ | ✅ | ❌ | |
| Docker support | ✅ | ✅ | ❌ | |
| Unit-testable (mocking)| ✅ | ❌ | ✅ | `MockCdpClient` for unit tests |

---

## pwright vs Playwright Differences

| Feature | Playwright | pwright |
|---------|:----------:|:-------:|
| Language Binding | TS/JS, Python, Java, C# | Rust |
| IPC / Transport Interface | Node server over JS interop | Native gRPC Protocol Buffers |
| CLI Capabilities | Native code generation / snapshots | Remote browser interaction over CLI |
| Component Focus | End-to-end testing monolithic engine | Highly modular and embedded automation |
| Memory Footprint | Heavy (spawns node processes) | Extremely lightweight (~5.9MB binary) |
| Chromium Distribution | Downloads and manages its own binaries | Connects to any pre-existing browser process |

---

## Playwright Parity Gaps

All previously identified parity gaps have been **implemented and verified**:

| # | Feature | Status | Golden Tests |
|---|---------|:------:|:------------:|
| 1 | File upload (`DOM.setFileInputFiles`) | ✅ | 1/1 |
| 2 | `get_by_text()` | ✅ | 2/2 |
| 3 | `get_by_label()` | ✅ | 2/2 |
| 4 | `get_by_role()` (with implicit role mapping) | ✅ | 3/3 |
| 5 | `filter({ hasText })` | ✅ | 1/1 |
| 6 | `and()` / `or()` composition | ✅ | 2/2 |
| 7 | Touchscreen (`Input.dispatchTouchEvent`) | ✅ | 1/1 |

### Intentionally Out of Scope

These are features from PinchTab or Playwright that pwright will **not** implement, as they fall outside pwright's role as a thin, embeddable browser bridge:

- **Human-like click/type** — PinchTab stealth concern (Bezier curves, variable delays)
- **Tab locking / mutex** — orchestration-layer concern
- **Tab screencast** — operational dashboard feature
- **Multi-instance orchestrator / Profile CRUD / Allocation policies** — control plane scope
- **Stealth mode / IDPI / API auth / Ad blocking** — security/anti-detection layer
- **Web dashboard / OpenAPI spec** — operational tooling
- **Browser lifecycle management** — pwright attaches to existing Chrome by design
- **Docker support** — pwright is a single binary; containerization is a deployment concern

---

## Summary

**Playwright** is the industry standard for end-to-end browser automation, heavily coupled with testing. It has everything from code generation to a massive distribution architecture. It operates fully client-server where the Node process governs browser lifecycles.

**PinchTab** is a batteries-included browser automation platform with an operational focus — Chrome lifecycle tracking, profiles, security, stealth, multi-instance orchestration, and a custom built dashboard using REST APIs.

**pwright** bridges the gap. It is a lightweight, **Playwright-compatible browser bridge** written in pure Rust. It mirrors Playwright's elegant API surface (`Page` → `Locator`) within a highly constrained (~5.9MB), dependency-free binary execution model. It communicates natively via cross-language gRPC or its CLI. It purposefully does not distribute browsers, handle Chromium lifecycles, or deal with orchestration—instead relying on attaching seamlessly to an already-running browser instance.
