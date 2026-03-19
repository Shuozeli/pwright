# Testing Strategy

pwright uses a two-tier testing approach: in-process fakes for fast unit tests,
and Docker-based integration tests for end-to-end verification.

---

## Tier 1: FakeCdpClient (in-process, no Chrome)

An in-memory browser simulation that CDP operations work against. Tests set up
pages with real HTML, and `Locator`/`Page` methods interact with a real DOM tree.

### Architecture

```
crates/pwright-fake/src/
  lib.rs           Crate root, re-exports
  client.rs        FakeCdpClient struct + CdpClient impl
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

`MockCdpClient` remains for call-sequence verification tests (e.g. "clicking
calls mousePressed then mouseReleased"). `FakeCdpClient` is for behavior
verification (e.g. "clicking a checkbox changes checked state").

| Test type | Use |
|-----------|-----|
| "Does click dispatch the right CDP calls?" | MockCdpClient |
| "Does is_checked() return true for a checked checkbox?" | FakeCdpClient |
| "Does first() pick the first of 3 elements?" | FakeCdpClient |
| "Does wait_for_response capture network events?" | MockCdpClient (event injection) |

---

## Tier 2: Docker Integration Tests (real Chrome, no internet)

End-to-end tests against a real Chrome headless instance. A fake HTTP server
provides test pages so no internet connection is needed.

### Architecture

```
tests/integration/
  docker-compose.yml           Chrome headless (CI)
  docker-compose.local.yml     Chrome headless (local dev)
  Dockerfile                   Test runner container
  Cargo.toml                   Integration test crate
  src/lib.rs                   Shared helpers (connect, navigate)
  pages/
    login.html                 Login form test page
    todo.html                  TodoMVC-like test page
    api-demo.html              SPA that makes API calls
  tests/
    actions.rs                 Click, fill, type, press, hover, select, drag
    click_navigation.rs        Click triggers <a> navigation
    concurrency.rs             5 parallel tabs, session isolation
    connection.rs              Connect, reconnect, health
    locator.rs                 Selector resolution + element queries
    locator_advanced.rs        getBy*, filter, composition, nth
    login.rs                   Full login flow
    navigation.rs              goto, reload, back/forward
    navigation_advanced.rs     Wait strategies, block images/media
    network.rs                 wait_for_response, response_body
    recipes.rs                 YAML recipe execution
    script_execution.rs        Script runner end-to-end
    chrome_http.rs             ChromeHttpClient + HTTP tab lifecycle
    coordinate_actions.rs      click-at, hover-at, dblclick coordinate commands
    network_capture.rs         network-listen, network-get, second CDP session
```

### Docker Compose

```yaml
services:
  chrome:
    image: chromedp/headless-shell:latest
    ports: ["9222:9222"]

  test-runner:
    build: .
    depends_on: [chrome]
```

### Test pages

| Page | Purpose | Key scenarios |
|------|---------|---------------|
| `login.html` | Form with email/password/submit | fill, click, is_disabled, wait_for |
| `todo.html` | Todo list with add/check/delete | first/last/nth, count, is_checked |
| `api-demo.html` | SPA making fetch() calls on load | wait_for_response, response_body |

### Running

```bash
# Local: start Chrome, run tests from host
docker compose -f tests/integration/docker-compose.local.yml up -d
cargo test -p pwright-integration-tests -- --ignored --test-threads=1

# CI: Dockerized test runner (matches CI)
cd tests/integration && docker compose up --build --abort-on-container-exit --exit-code-from test-runner
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
2. 96 integration tests across 15 test files
3. Concurrency tests (6 Docker + 6 FakeCdpClient)
4. Coordinate action tests (click-at, hover-at, dblclick)
5. Network capture tests (network-listen, network-get)
6. CI: Dockerized test runner (`docker compose up --abort-on-container-exit`)
7. Two compose files: `docker-compose.yml` (CI) + `docker-compose.local.yml` (dev)
