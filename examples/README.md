# pwright Examples

Rust equivalents of the [Playwright examples](../playwright/examples/),
demonstrating `pwright-bridge` API usage.

Each example maps 1:1 to the original Playwright test, with the original
JS/TS code shown as a doc comment above each Rust test.

## Examples

| Crate | Tests | Description |
|-------|-------|-------------|
| `example-todomvc` | 23 | Add, complete, delete, edit, filter, create todos |
| `example-svgomg` | 6 | Menu, defaults, features, reset, download, open svg |
| `example-mock-battery` | 4 | Mock `navigator.getBattery`, verify updates + API calls |
| `example-mock-filesystem` | 2 | Mock file/directory pickers |
| `example-github-api` | 2 | API testing via `evaluate(fetch(...))` |

## Running Tests

```bash
# All examples
cargo test -p example-todomvc -p example-svgomg -p example-mock-battery \
           -p example-mock-filesystem -p example-github-api

# Single example
cargo test -p example-todomvc

# Single test file
cargo test -p example-todomvc --test adding_todos
```

## Structure

Each follows the same pattern:

```
examples/<name>/
├── Cargo.toml          # depends on pwright-bridge
├── tests/
│   └── <test_name>.rs  # Rust integration tests
└── [assets/]           # HTML/JS demo assets (if applicable)
```

Tests use `MockCdpClient` to verify correct CDP method calls without
requiring a real browser.
