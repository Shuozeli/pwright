# gRPC API Reference

Service: `pwright.v1.BrowserService`
Proto: [`proto/pwright/v1/browser.proto`](../proto/pwright/v1/browser.proto)

---

## Browser Lifecycle

### `ConnectBrowser`

Connect to a running Chrome instance via CDP WebSocket.

```protobuf
rpc ConnectBrowser(ConnectBrowserRequest) returns (ConnectBrowserResponse);
```

| Field | Type | Description |
|-------|------|-------------|
| `cdp_url` | string | Chrome DevTools WebSocket URL |

### `Health`

Health check — reports connection status and open tab count.

```protobuf
rpc Health(HealthRequest) returns (HealthResponse);
```

---

## Tab Management

### `CreateTab`

Open a new browser tab.

| Field | Type | Description |
|-------|------|-------------|
| `url` | string | Initial URL (default: `about:blank`) |

**Returns**: `tab_id`, `url`, `title`

### `CloseTab`

Close a tab by ID.

| Field | Type | Description |
|-------|------|-------------|
| `tab_id` | string | Tab ID from `CreateTab` or `Navigate` |

### `ListTabs`

List all open page targets.

**Returns**: Array of `TabInfo { target_id, type, title, url }`

### `BringToFront`

Activate a tab (bring it to foreground).

| Field | Type | Description |
|-------|------|-------------|
| `tab_id` | string | Tab ID to activate |

---

## Navigation

### `Navigate`

Navigate a tab to a URL with configurable wait strategies.

| Field | Type | Description |
|-------|------|-------------|
| `tab_id` | string | Target tab (empty = current) |
| `url` | string | Destination URL |
| `new_tab` | bool | Create new tab before navigating |
| `timeout_ms` | double | Navigation timeout (default: 30000) |
| `wait_for` | WaitStrategy | `WAIT_NONE`, `WAIT_DOM`, `WAIT_NETWORK_IDLE`, `WAIT_SELECTOR` |
| `wait_selector` | string | CSS selector (when `wait_for = WAIT_SELECTOR`) |
| `block_images` | bool | Block image requests |
| `block_media` | bool | Block all media requests |

**Returns**: `tab_id`, final `url`, `title`

### `Reload`

Reload the current page and wait for DOM readiness.

| Field | Type | Description |
|-------|------|-------------|
| `tab_id` | string | Target tab |

### `GoBack`

Navigate back in history (`history.back()`).

| Field | Type | Description |
|-------|------|-------------|
| `tab_id` | string | Target tab |

### `GoForward`

Navigate forward in history (`history.forward()`).

| Field | Type | Description |
|-------|------|-------------|
| `tab_id` | string | Target tab |

---

## Content Extraction

### `GetSnapshot`

Get the accessibility tree as a flat list of nodes with stable refs.

| Field | Type | Description |
|-------|------|-------------|
| `tab_id` | string | Target tab |
| `filter` | SnapshotFilter | `FILTER_ALL` or `FILTER_INTERACTIVE` |
| `max_depth` | int32 | Max tree depth (-1 = unlimited) |

**Returns**: Array of `A11yNode { ref, role, name, depth, value, disabled, focused, node_id }`

The `ref` field (e.g., `e5`) can be used in `ExecuteAction` to target elements.

### `TakeScreenshot`

Capture the current page as an image.

| Field | Type | Description |
|-------|------|-------------|
| `tab_id` | string | Target tab |
| `format` | string | `"png"`, `"jpeg"`, `"webp"` |
| `quality` | int32 | 0-100 (for jpeg/webp) |
| `full_page` | bool | Capture beyond viewport |

**Returns**: Raw image bytes in `data`

### `GetText`

Extract visible text content from the page.

| Field | Type | Description |
|-------|------|-------------|
| `tab_id` | string | Target tab |

**Returns**: `text` — the page's `document.body.innerText`

### `GetPDF`

Print the page to PDF.

| Field | Type | Description |
|-------|------|-------------|
| `tab_id` | string | Target tab |

**Returns**: Raw PDF bytes in `data`

---

## Actions

### `ExecuteAction`

Perform a browser action on an element.

| Field | Type | Description |
|-------|------|-------------|
| `tab_id` | string | Target tab |
| `kind` | ActionKind | See below |
| `ref` | string | Element ref from `GetSnapshot` (e.g., `e5`) |
| `selector` | string | CSS selector (alternative to ref) |
| `text` | string | Text for `TYPE`/`FILL` |
| `key` | string | Key name for `PRESS` (e.g., `Enter`, `Tab`, `ArrowDown`) |
| `value` | string | Value for `SELECT` |
| `scroll_x/y` | int32 | Scroll offset for `SCROLL` |
| `drag_x/y` | int32 | Drag delta for `DRAG` |
| `wait_nav` | bool | Wait for navigation after action |

**ActionKind values:**

| Kind | What it does |
|------|-------------|
| `CLICK` | Click element center (scrollIntoView → focus → mousePress/Release) |
| `TYPE` | Type text character by character into focused element |
| `FILL` | Set element value programmatically + dispatch input/change events |
| `PRESS` | Dispatch a named key (Enter, Tab, Escape, arrows, F1-F12) |
| `FOCUS` | Focus an element |
| `HOVER` | Move mouse to element center |
| `SELECT` | Set value of a `<select>` element |
| `SCROLL` | Scroll element into view, or scroll page by offset |
| `DRAG` | Drag element by (dx, dy) pixels |
| `CHECK` | Click checkbox to check it (scrollIntoView → click) |
| `UNCHECK` | Click checkbox to uncheck it (scrollIntoView → click) |
| `DBLCLICK` | Double-click element center (two press/release pairs with clickCount) |

### `SetInputFiles`

Set files on a file input element.

| Field | Type | Description |
|-------|------|-------------|
| `tab_id` | string | Target tab |
| `ref` | string | Element ref from `GetSnapshot` |
| `selector` | string | CSS selector (alternative to ref) |
| `files` | string[] | Absolute file paths on the server |

### `TouchTap`

Dispatch a touch tap at coordinates.

| Field | Type | Description |
|-------|------|-------------|
| `tab_id` | string | Target tab |
| `x` | double | X coordinate |
| `y` | double | Y coordinate |

### `ExpectDownload`

Trigger an action and capture its download.

| Field | Type | Description |
|-------|------|-------------|
| `tab_id` | string | Target tab |
| `action` | ExecuteActionRequest | Click/Press action that triggers the download |

**Returns**: `file_path` — absolute path to the downloaded file on the server

---

## JavaScript

### `Evaluate`

Evaluate a JavaScript expression in the page context.

| Field | Type | Description |
|-------|------|-------------|
| `tab_id` | string | Target tab |
| `expression` | string | JS expression |

**Returns**: `result` — JSON-encoded evaluation result

> **⚠️ Security**: This RPC executes arbitrary JavaScript in the page context. Any gRPC client can read cookies, modify the DOM, or make network requests as the page. Use `--disable-eval` to block this RPC in sensitive deployments.

---

## Cookies

### `GetCookies`

Get all cookies for the current page.

### `SetCookies`

Set cookies. Each `CookieEntry` has: `name`, `value`, `domain`, `path`, `expires`, `http_only`, `secure`, `same_site`.
