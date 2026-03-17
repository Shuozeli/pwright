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
- `with_page` was removed - callers manage tab lifecycle via `Browser::new_tab` / `TabHandle::close`
- Do NOT change `Page::close()` to `&mut self` - uses AtomicBool for Arc sharing
- Do NOT revert CancellationToken to `drop(writer_handle)` in CdpConnection
- Do NOT change `nodeId` back to `backendNodeId` in DOM methods
- Use `serde_json::to_string()` for JS string escaping (not manual `replace`)
- Use `r##"..."##` for raw strings containing `@`, `#` in tests (Rust 2024 edition)
- Do NOT use `#[allow(...)]` to bypass clippy or rustdoc warnings - fix the root cause instead (escape doc comments, fix the code, etc.)
- pwright-bridge is a **stateless library** - it does NOT track tabs, manage pools, or do implicit cleanup. Callers own tab lifecycle. Do NOT propose automatic tab leak detection, tab garbage collection, or drop guards that implicitly close tabs. The `with_page` mistake (implicit lifecycle) must not be repeated.

## Code Quality Discipline

When implementing a feature or fix, re-read your own diff before declaring
it done. Do not silently leave shortcuts in the code. Specifically:

- **Flag known shortcuts** with `// TODO(refactor):` comments. Examples:
  `.clone()` where `.take()` would work, duplicated patterns that should
  be extracted, silently swallowed errors via `unwrap_or_default()`.
- **Do not duplicate code 3+ times** without extracting a helper. If you
  find yourself writing the same pattern a third time, stop and refactor.
- **Do not silently swallow errors.** If you write `let _ = ...` or
  `.unwrap_or_default()`, add a comment explaining why it's safe, or
  propagate the error properly.
- **Self-review pass:** After getting the feature working and tests passing,
  re-read the full diff once. Fix obvious issues (unnecessary clones,
  missing error context, copy-pasted code). This is cheaper than a
  separate review pass later.

This rule exists because implementation-mode focus ("make it work") tends to
accumulate silent tech debt that only surfaces in dedicated code reviews.

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

## CI

After pushing, check GitHub Actions status:
```bash
gh run list --limit 3
gh run view <run-id>        # if a run fails
```

## Key Docs

- `docs/index.md` - Full project guide for agents
- `docs/known-issues.md` - Prioritized bug/improvement list
- `docs/knowledge/script-runner-design.md` - Script runner phases
- `docs/knowledge/testing-strategy.md` - Test architecture
