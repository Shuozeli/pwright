# Codelabs: pwright Usage Guide

Practical examples for automating Chrome with pwright. Each codelab shows the **Rust API** (via `pwright-bridge`), the equivalent **CLI** command, and the equivalent **gRPC** call.

---

## 1. Navigate and Extract Content

### Rust (Playwright API)

```rust
use pwright_bridge::playwright::Page;

let page = Page::new(session);

// Navigate
page.goto("https://example.com", None).await?;

// Extract content
let title = page.title().await?;
let url = page.url().await?;
let html = page.content().await?;
let text = page.locator("h1").text_content().await?;
println!("Title: {title}, URL: {url}");
```

### CLI

```bash
pwright open https://example.com
pwright snapshot            # Get accessibility tree
pwright eval "document.title"
```

### gRPC

```bash
grpcurl -d '{"url":"https://example.com","new_tab":true}' localhost:50051 pwright.v1.BrowserService/Navigate
grpcurl -d '{"tab_id":"..."}' localhost:50051 pwright.v1.BrowserService/GetText
grpcurl -d '{"tab_id":"...","expression":"document.title"}' localhost:50051 pwright.v1.BrowserService/Evaluate
```

---

## 2. Locators: Finding Elements

pwright supports multiple locator strategies, mirroring Playwright's API:

### CSS Selectors (simplest)

```rust
let locator = page.locator("button.submit");
locator.click().await?;
```

### By Text (substring or exact)

```rust
// Substring match
let el = page.get_by_text("Sign up", false);
el.click().await?;

// Exact match
let el = page.get_by_text("Sign up", true);
```

> **Design decision**: `get_by_text` uses JS-based resolution via `Runtime.evaluate` since CSS cannot match by text content. The element is found in JS, then resolved to a `nodeId` via `DOM.requestNode`.

### By Label

```rust
let input = page.get_by_label("Email Address");
input.fill("user@example.com").await?;
```

Looks up `<label for="...">`, wrapping `<label>`, and `aria-label` attributes.

### By Role

```rust
// Find button named "Submit"
let btn = page.get_by_role("button", Some("Submit"));
btn.click().await?;

// Find any heading
let h = page.get_by_role("heading", None);
let text = h.text_content().await?;
```

Supports 18 implicit role mappings (e.g., `<button>` → `button`, `<a href>` → `link`, `<h1>` → `heading`, `<input type="text">` → `textbox`).

### By Test ID / Placeholder / Alt Text / Title

```rust
page.get_by_test_id("login-btn").click().await?;
page.get_by_placeholder("Search...").fill("query").await?;
page.get_by_alt_text("Logo").click().await?;
page.get_by_title("Close dialog").click().await?;
```

These all use CSS attribute selectors: `[data-testid="..."]`, `[placeholder="..."]`, `[alt="..."]`, `[title="..."]`.

---

## 3. Locator Composition

### Filter by text

```rust
let items = page.locator(".item");
let banana = items.filter_by_text("Banana");
banana.click().await?;
```

### Combine with AND / OR

```rust
let primary_btns = page.locator("button").and(&page.locator(".primary"));
let any_action = page.locator("button").or(&page.locator("a.action"));
```

- **`and()`** uses CSS `:is()` — matches elements satisfying both selectors
- **`or()`** uses CSS union (`,`) — matches elements satisfying either selector

---

## 4. Input: Keyboard, Mouse, Touchscreen

### Keyboard

```rust
let kb = page.keyboard();

// Type text character by character
kb.type_text("Hello World").await?;

// Press special keys
kb.press("Enter").await?;
kb.press("Control+a").await?;  // Select all

// Low-level key events
kb.down("Shift").await?;
kb.press("ArrowDown").await?;
kb.up("Shift").await?;
```

### Mouse

```rust
let mouse = page.mouse();

mouse.click(100.0, 200.0, None).await?;
mouse.dblclick(100.0, 200.0).await?;
mouse.move_to(300.0, 400.0).await?;
mouse.wheel(0, -300).await?; // Scroll up

// Right-click
mouse.click(100.0, 200.0, Some(ClickOptions {
    button: Some("right".to_string()),
    ..Default::default()
})).await?;
```

### Touchscreen

```rust
let touch = page.touchscreen();
touch.tap(150.0, 250.0).await?;  // Touch tap
```

> `tap()` dispatches `touchStart` + `touchEnd` via `Input.dispatchTouchEvent` CDP.

### CLI equivalents

```bash
pwright click e1           # Click element by ref
pwright dblclick e1        # Double-click element by ref
pwright type "Hello"       # Type text
pwright press Enter        # Press key
pwright fill e2 "value"    # Fill input
pwright hover e3           # Hover element

# Coordinate-based (for SPAs with empty accessibility trees)
pwright click-at 282 293              # Click at (x, y)
pwright click-at 282 293 --button right   # Right-click
pwright click-at 282 293 --click-count 2  # Double-click
pwright hover-at 282 293              # Hover at (x, y)
```

---

## 5. File Upload

```rust
let file_input = page.locator("input[type='file']");
file_input.set_input_files(vec!["/path/to/file.pdf".to_string()]).await?;
```

Uses `DOM.setFileInputFiles` CDP command under the hood.

---

## 6. File Download

### Rust

```rust
let path = page.expect_download(|| async {
    page.locator("a.download-link").click().await
}).await?;
println!("Downloaded to: {}", path);
```

### CLI

```bash
pwright download e5 --dest ./report.pdf
```

---

## 7. Screenshots and PDF

### Rust

```rust
use pwright_bridge::playwright::ScreenshotOptions;

// Full page PNG
let data = page.screenshot(Some(ScreenshotOptions {
    format: Some("png".into()),
    full_page: true,
    ..Default::default()
})).await?;

// PDF export
let pdf = page.pdf().await?;
```

### CLI

```bash
pwright screenshot --filename page.png
pwright pdf --filename page.pdf
```

---

## 8. Tab Management

### Rust (via Browser)

```rust
// List tabs
let tabs = browser.list_tabs().await?;

// Create new tab
let tab_id = browser.create_tab("https://example.com").await?;

// Switch tab
browser.bring_to_front(&tab_id).await?;

// Close tab
browser.close_tab(&tab_id).await?;
```

### CLI

```bash
pwright tab-list
pwright tab-new https://example.com
pwright tab-select <tab_id>
pwright tab-close <tab_id>
```

---

## 9. Cookie Management

### Rust

```rust
let cookies = browser.get_cookies(&tab_id).await?;
browser.set_cookies(&tab_id, cookies).await?;
```

### CLI

```bash
pwright cookie-list    # Lists cookies for active tab
pwright cookie-set --name session --value abc123 --domain example.com
```

---

## 10. JavaScript Evaluation

### Rust

```rust
// Raw evaluate (returns serde_json::Value)
let result = page.evaluate("1 + 1").await?;

// Typed evaluate_into (generic, returns concrete types)
let title: String = page.evaluate_into("document.title").await?;
let count: i64 = page.evaluate_into("document.querySelectorAll('a').length").await?;
let ready: bool = page.evaluate_into("!!document.querySelector('.loaded')").await?;

// Structured JSON data via FromEvalJson wrapper
use pwright_bridge::FromEvalJson;
let items: FromEvalJson<Vec<Item>> = page.evaluate_into("JSON.stringify([...])").await?;
let data = items.0;

// Async evaluation (awaits Promises)
let text: String = page.evaluate_async_into("fetch('/api').then(r => r.text())").await?;
```

### CLI

```bash
pwright eval "document.title"
pwright eval "document.querySelectorAll('a').length"
```

---

## Design Decisions

### JS-Based Selector Resolution

For `get_by_text`, `get_by_label`, and `get_by_role`, pwright cannot use CSS selectors because CSS has no native text-content or role-matching capability. Instead, these selectors use a **JS bridge approach**:

1. A special prefix (e.g., `__pw_text=`, `__pw_label=`, `__pw_role=`) is embedded in the selector string
2. `selectors.rs` detects the prefix and delegates to `Runtime.evaluate`
3. The JS code finds the matching element and returns its `objectId`
4. `DOM.requestNode` converts the `objectId` to a `nodeId` for CDP operations

This keeps the `Locator` type simple (just a selector string + session + clock reference) while supporting rich matching beyond CSS.

### Selector Composition Strategy

- **`and()`**: CSS `:is()` pseudo-class for same-element intersection (`button:is(.primary)`)
- **`or()`**: CSS union via comma (`button, a.action`)
- **`filter_by_text()`**: JS-based post-filter using `__pw_filter_text` prefix

### Touchscreen via CDP

Touchscreen uses `Input.dispatchTouchEvent` rather than synthesizing via mouse events. This ensures proper `touchstart`/`touchend` event sequences on mobile-emulated pages.

### File Upload

Uses `DOM.setFileInputFiles` CDP command, which is the only way to set files on `<input type="file">` elements via CDP (direct JS assignment to `FileList` is not possible).

---

## 11. Script Runner

pwright supports declarative YAML scripts that define multi-step browser
automation workflows. Scripts take parameters, execute steps sequentially,
and produce structured JSONL output.

### Basic Script

```yaml
# hello.yaml
name: "Extract page title"
params:
  url:
    type: string
    required: true

steps:
  - goto: "{{ url }}"
  - extract:
      selector: "h1"
      field: text_content
      save_as: heading
  - output:
      heading: "{{ heading }}"
```

```bash
pwright script run hello.yaml --param url=https://example.com
```

Output:
```jsonl
{"step_index":0,"step_type":"goto","status":"ok","duration_ms":1200,"details":{"url":"https://example.com"}}
{"step_index":1,"step_type":"extract","status":"ok","duration_ms":5,"details":{"value":"Example Domain"}}
{"step_index":2,"step_type":"output","status":"ok","duration_ms":0,"details":{"heading":"Example Domain"}}
{"summary":true,"name":"Extract page title","status":"ok","total_steps":3,"succeeded":3}
```

### JS Script Registry

Complex JavaScript is defined centrally in a `scripts` section, not inline:

```yaml
name: "Link extractor"
scripts:
  extract_links: |
    [...document.querySelectorAll('a[href]')]
      .map(a => ({ text: a.textContent.trim(), href: a.href }))
      .filter(l => l.href.startsWith('http'))

steps:
  - goto: "{{ url }}"
  - eval:
      ref: extract_links
      save_as: links
  - output:
      links: "{{ links }}"
```

Scripts can accept arguments:

```yaml
scripts:
  click_nth: |
    function(index) {
      document.querySelectorAll('.item')[index].click();
    }

steps:
  - eval: { ref: click_nth, args: ["3"] }
```

### Parameters and Secrets

```bash
# Inline params
pwright script run script.yaml --param url=https://example.com --param max=10

# From file (for credentials)
pwright script run script.yaml --param-file secrets.yaml
```

```yaml
# secrets.yaml
email: "user@example.com"
password: "secret123"
```

### Error Handling

Each step can specify what happens on failure:

```yaml
steps:
  - click: ".optional-button"
    on_error: continue          # skip if not found

  - extract:
      selector: "#required"
      field: text_content
      save_as: data
    on_error: fail              # stop script (default)
```

### Validation

```bash
# Check script without executing
pwright script validate script.yaml --param url=https://example.com
```

Validates: required params supplied, template references resolve,
JS registry refs exist, selectors non-empty.

### Extract Fields

The `extract` step supports these field types:

| Field | Description |
|-------|-------------|
| `text_content` | Element's textContent |
| `inner_text` | Element's innerText (layout-aware) |
| `inner_html` | Element's innerHTML |
| `input_value` | Input/textarea value |
| `is_visible` | "true"/"false" |
| `is_checked` | "true"/"false" |
| `is_disabled` | "true"/"false" |
| `attribute:<name>` | Named attribute (e.g. `attribute:href`) |

### Sample Scripts

See [examples/scripts/](../examples/scripts/) for complete working examples:

- **hello.yaml** -- minimal navigate + extract
- **extract-links.yaml** -- JS registry with args
- **login-and-scrape.yaml** -- multi-step with credentials
- **form-fill.yaml** -- fill and submit
- **screenshot-audit.yaml** -- page structure audit
