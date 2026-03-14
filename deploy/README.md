# pwright Deployment

One-command setup: Chrome headless + pwright gRPC server.

## Quick Start

```bash
cd deploy
docker compose up --build -d

# gRPC is now available at localhost:50051
```

## Using the gRPC API

```bash
# Navigate to a page
grpcurl -plaintext -d '{"url":"https://example.com","new_tab":true}' \
  localhost:50051 pwright.v1.BrowserService/Navigate

# Take a snapshot
grpcurl -plaintext -d '{"tab_id":"..."}' \
  localhost:50051 pwright.v1.BrowserService/GetSnapshot

# Click an element
grpcurl -plaintext -d '{"tab_id":"...","kind":"CLICK","ref":"e1"}' \
  localhost:50051 pwright.v1.BrowserService/ExecuteAction
```

## Running YAML Scripts

Mount your scripts into `/scripts` and use the CLI inside the container:

```bash
# Run the bundled hello script
docker compose exec pwright \
  pwright --cdp http://chrome:9222 script run /scripts/hello.yaml \
  --param url=https://example.com

# Mount your own scripts directory
docker compose run -v ./my-scripts:/scripts pwright \
  pwright --cdp http://chrome:9222 script run /scripts/my-scraper.yaml
```

## Architecture

```
Host                          Docker Network
  :50051 ◄──── gRPC ────── pwright-server ──── CDP/WS ────── chrome
                            (Rust binary)                    (headless)
```

Both containers share a Docker network. pwright connects to Chrome via
the `chrome` hostname. The gRPC port (50051) is exposed to the host.

## Configuration

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| `CDP_HOST` | `chrome` | Chrome hostname (Docker service name) |
| `CDP_PORT` | `9222` | Chrome CDP port |

## Stopping

```bash
docker compose down
```

## TODO

- [ ] Publish pre-built Docker image to ghcr.io (eliminates build step)
- [ ] Add `--upload-dir` flag for file upload path restriction
- [ ] Health check endpoint for container orchestration
