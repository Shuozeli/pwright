# Testing Strategy

pwright uses a two-tier testing approach: in-process fakes for fast unit tests,
and Docker-based integration tests for end-to-end verification.

---

## Tier 1: FakeCdpClient (in-process, no Chrome)

An in-memory browser simulation that CDP operations work against. Tests set up
pages with real HTML, and `Locator`/`Page` methods interact with a real DOM tree.

### Architecture

```
crates/pwright-bridge/src/fake/
  mod.rs           FakeCdpClient struct + CdpClient impl
  dom.rs           In-memory DOM tree (nodes, attributes, text)
  selector.rs      CSS selector matching against the DOM
  builder.rs       HTML parser for test setup
```

### Usage

```rust
let fake = FakeCdpClient::from_html(r#"
    <div id="app">
        <h1>Welcome</h1>
        <input type="checkbox" id="agree" checked />
        <button class="submit" disabled>Submit</button>
        <ul>
            <li>Item 1</li>
            <li>Item 2</li>
        </ul>
    </div>
"#);

let page = Page::new(Arc::new(fake));
assert!(page.is_visible("button.submit").await?);
assert!(page.locator("#agree").is_checked().await?);
assert_eq!(page.text_content("h1").await?, Some("Welcome".into()));
```

### What the fake simulates

| CDP method | Fake behavior |
|-----------|--------------|
| `dom_get_document` | Return root node ID from in-memory tree |
| `dom_query_selector` | Match CSS against in-memory DOM |
| `dom_query_selector_all` | Return all matching node IDs |
| `dom_get_attributes` | Return node's attributes from tree |
| `dom_get_outer_html` | Serialize node to HTML string |
| `dom_get_box_model` | Succeed if node is visible, fail if hidden |
| `dom_resolve_node` | Map nodeId to objectId |
| `dom_request_node` | Map objectId to nodeId |
| `runtime_call_function_on` | Evaluate simple property checks (checked, disabled, textContent) |
| `runtime_evaluate` | Configurable (for complex JS, selector resolution) |

### CSS selector matching scope

- Tag: `button`, `div`, `li`
- Class: `.submit`, `.toggle`
- ID: `#agree`
- Attribute presence: `[checked]`, `[disabled]`
- Attribute value: `[type="checkbox"]`, `[data-testid="x"]`
- Descendant: `div span`, `ul li`
- Compound: `button.submit`, `input[type="checkbox"]`

### What stays configurable

- `runtime_evaluate` for arbitrary JS (page queries, selector resolution)
- Screenshots, PDF, cookies (don't interact with DOM)
- Event injection (`send_event`)
- Navigation responses

### Coexistence with MockCdpClient

`MockCdpClient` (renamed `StubCdpClient`) remains for call-sequence verification
tests (e.g. "clicking calls mousePressed then mouseReleased"). `FakeCdpClient`
is for behavior verification (e.g. "clicking a checkbox changes checked state").

| Test type | Use |
|-----------|-----|
| "Does click dispatch the right CDP calls?" | StubCdpClient |
| "Does is_checked() return true for a checked checkbox?" | FakeCdpClient |
| "Does first() pick the first of 3 elements?" | FakeCdpClient |
| "Does wait_for_response capture network events?" | StubCdpClient (event injection) |

---

## Tier 2: Docker Integration Tests (real Chrome, no internet)

End-to-end tests against a real Chrome headless instance. A fake HTTP server
provides test pages so no internet connection is needed.

### Architecture

```
tests/
  docker-compose.yml       Chrome headless + fake HTTP server
  fixtures/
    server/                Axum-based fake HTTP server
      Cargo.toml
      src/main.rs          Serves static + dynamic routes
    pages/
      login.html           Login form test page
      todo.html            TodoMVC-like test page
      api.html             SPA that makes API calls
  integration/
    navigation.rs          Real goto, reload, back/forward
    locator.rs             Real selector resolution + actions
    network.rs             Real wait_for_response + response_body
```

### Docker Compose

```yaml
services:
  chrome:
    image: chromium/chrome:latest
    command: --headless --remote-debugging-port=9222 --no-sandbox
    ports: ["9222:9222"]

  fake-server:
    build: fixtures/server
    ports: ["3000:3000"]
```

### Test pages

| Page | Purpose | Key scenarios |
|------|---------|---------------|
| `login.html` | Form with email/password/submit | fill, click, is_disabled, wait_for |
| `todo.html` | Todo list with add/check/delete | first/last/nth, count, is_checked |
| `api.html` | SPA making fetch() calls on load | wait_for_response, response_body |

### Running

```bash
# Start test infrastructure
docker compose -f tests/docker-compose.yml up -d

# Run integration tests
cargo test --test integration -- --ignored

# Tear down
docker compose -f tests/docker-compose.yml down
```

---

## Implementation Status

### Phase 1: FakeCdpClient -- DONE
1. In-memory DOM tree with HTML parser (`pwright-fake`)
2. CSS selector matching engine (tag, class, id, attribute, descendant, compound)
3. FakeCdpClient implementing CdpClient with real DOM operations
4. 19 behavior verification tests (`fake_browser.rs`)
5. 3 examples migrated to FakeCdpClient (todomvc, svgomg, mock-filesystem)
6. MockCdpClient retained for CDP call-sequence verification

### Phase 2: Docker Integration -- DONE
1. Docker compose with Chrome (`chromedp/headless-shell`) + in-process test server
2. 58 integration tests across 9 test files
3. Concurrency tests (6 Docker + 6 FakeCdpClient)
4. CI: Dockerized test runner (`docker compose up --abort-on-container-exit`)
5. Two compose files: `docker-compose.yml` (CI) + `docker-compose.local.yml` (dev)
