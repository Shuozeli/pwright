# pwright Improvements

Proposed improvements organized by priority. Updated 2026-03-21.

## P0: Agent Experience

### 1. Recipe auto-discovery (`pwright recipe list`)
Output JSON describing all available recipes with names, params, auth
requirements, and descriptions. Agents pick the right recipe without
reading YAML files.

```bash
pwright recipe list                    # JSON array of all recipes
pwright recipe list --category social  # filter by category
```

### 2. `page.wait_for_text(text, timeout_ms)` -- DONE
Implemented in bridge (`Page::wait_for_text`, `Page::wait_until`) and CLI
(`wait-for-text`, `wait-for`, `wait-until`).

### 3. Streaming `RunScript` gRPC RPC
Execute recipes remotely via gRPC without CLI/SSH. Server-side streaming
returns step results as they complete.

Already in `docs/todo.md` as P1.

## P1: Reliability

### 4. Configurable retry on step failure
Currently steps fail immediately on selector-not-found. SPAs often need
1-2 retries as content loads lazily. Add a per-step `retry` option:

```yaml
steps:
  - click: ".dynamic-button"
    retry: 3              # retry up to 3 times
    retry_delay_ms: 1000  # wait 1s between retries
```

Default: no retry (current behavior). Configurable per step.

### 5. Recipe freshness check (`pwright recipe test`)
Run a recipe and validate the output isn't empty, flagging stale
selectors before agents hit them at runtime.

```bash
pwright recipe test examples/recipes/research/social/reddit-feed.yaml
# OK: 15 posts extracted
# or
# STALE: 0 posts extracted (selector may have changed)
```

## P2: Developer Experience

### 6. `pwright explore <url>`
Dump a structured analysis of a page: key elements with data-testid,
forms, tables, and suggested extraction JS. Cuts exploration from
15 minutes to 30 seconds.

```bash
pwright explore https://news.ycombinator.com
# {
#   "title": "Hacker News",
#   "forms": [],
#   "tables": [{"selector": ".itemlist", "rows": 30}],
#   "testids": [],
#   "interactive": ["input#hnmain", "a.storylink x30"],
#   "suggested_extraction": "..."
# }
```

### 7. Selector playground (`pwright select`)
Quick selector testing without writing JS.

```bash
pwright select ".athing"          # 30 matches
pwright select ".athing" --first  # first match text content
pwright select "[data-testid]"    # list all testid values
```

### 8. `pwright recipe run` shorthand
Auto-discover recipe by name from `examples/recipes/`.

```bash
pwright recipe run zhihu-hot                    # finds the YAML
pwright recipe run reddit-feed --param subreddit=r/rust
```

### 9. Recipe template generator (`pwright recipe init`)
Navigate, snapshot, generate skeleton YAML.

```bash
pwright recipe init https://example.com --name "Example Feed"
# Creates example-feed.yaml with URL, wait_for, empty extraction
```

### 10. Script variables in recipe name
Allow `{{ var }}` in the `name` field for descriptive output.

```yaml
name: "Reddit Feed ({{ subreddit }})"
```

## P3: Infrastructure

### 11. Dry-run mode
`pwright script run --dry-run` shows steps without executing.

### 12. `--output json` for CLI commands
Machine-parseable output for `snapshot`, `eval`, etc.

### 13. Recipe versioning
Track which recipe version produced which output.

### 14. Parallel recipe execution
Run multiple recipes concurrently in separate tabs.

## Rejected

| Proposal | Reason |
|----------|--------|
| Cookie persistence across sessions | Security risk — cookies contain auth tokens. Each session should require explicit login. |
