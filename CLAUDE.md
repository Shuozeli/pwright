# pwright

Lightweight Rust CDP bridge for browser automation. Connects to Chrome via
WebSocket, exposes Playwright-compatible Rust API + gRPC + CLI.

## Quick Orientation

Read `docs/index.md` for the full project guide, architecture decisions,
and crate map. That file is the authoritative reference for agents.

## Git Rules

- Do NOT commit or push unless the user explicitly asks you to
- Do NOT amend commits unless the user explicitly asks you to
- Do NOT force push unless the user explicitly asks you to

## Key Rules

- Do NOT propose adding browser launch/install - pwright is attach-only by design
- Do NOT propose adding gRPC authentication - documented design choice
- Do NOT change `with_page` to register in tab map - ephemeral by design
- Do NOT change `Page::close()` to `&mut self` - uses AtomicBool for Arc sharing
- Do NOT revert CancellationToken to `drop(writer_handle)` in CdpConnection
- Do NOT change `nodeId` back to `backendNodeId` in DOM methods
- Use `serde_json::to_string()` for JS string escaping (not manual `replace`)
- Use `r##"..."##` for raw strings containing `@`, `#` in tests (Rust 2024 edition)

## Build & Test

```bash
cargo test --workspace                    # unit + fake tests
cargo doc --workspace --no-deps           # docs (must pass -D warnings)

# Integration tests (local)
docker compose -f tests/integration/docker-compose.local.yml up -d
cargo test -p pwright-integration-tests -- --ignored --test-threads=1

# Integration tests (Docker, matches CI)
cd tests/integration && docker compose up --build --abort-on-container-exit --exit-code-from test-runner
```

## Key Docs

- `docs/index.md` - Full project guide for agents
- `docs/known-issues.md` - Prioritized bug/improvement list
- `docs/knowledge/script-runner-design.md` - Script runner phases
- `docs/knowledge/testing-strategy.md` - Test architecture
