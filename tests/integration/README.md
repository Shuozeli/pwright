# Integration Tests

End-to-end tests against a real Chrome headless instance.
The test server runs in-process within the test-runner container.
No internet connection required.

## Prerequisites

- Docker and Docker Compose

## Running

### Docker (recommended, matches CI)

```bash
# Run everything in Docker (Chrome + test runner)
docker compose -f tests/integration/docker-compose.yml up --build --abort-on-container-exit --exit-code-from test-runner

# Tear down
docker compose -f tests/integration/docker-compose.yml down
```

### Local (requires Chrome running separately)

```bash
# Start Chrome headless
docker compose -f tests/integration/docker-compose.yml up -d chrome

# Wait for Chrome, then run tests from host
sleep 3
cargo test -p pwright-integration-tests -- --ignored --test-threads=1

# Tear down
docker compose -f tests/integration/docker-compose.yml down
```

## Architecture

```
docker-compose.yml
  chrome          chromedp/headless-shell (CDP on :9222)
  test-runner     Rust test binary (built from Dockerfile)
                  Starts in-process axum test server
                  Connects to chrome:9222 via Docker network

Dockerfile        Builds the integration test binary

src/lib.rs        Shared helpers:
                  - SharedCdp: one WS connection per process
                  - Test server: axum on 0.0.0.0 (Docker) or 127.0.0.1 (local)
                  - connect_and_navigate(): creates tab, navigates, returns Page

tests/
  connection.rs   CDP connection lifecycle + concurrency
  concurrency.rs  Parallel tab operations
  locator.rs      DOM queries, form interactions, click
  login.rs        Full login flow with JS validation
  navigation.rs   goto, title, url, content
  network.rs      wait_for_response, response_body, evaluate_async

pages/
  login.html      Login form with JS validation
  todo.html       TodoMVC-like page
  api-demo.html   SPA that fetches /api/search on load
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `CHROME_HOST` | `localhost` | Chrome CDP hostname |
| `CHROME_PORT` | `9222` | Chrome CDP port |

In Docker, these are set by docker-compose.yml (`chrome:9222`).
Locally, they default to `localhost:9222`.
