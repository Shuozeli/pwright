# pwright Documentation

Lightweight Rust CDP bridge exposing Chrome browser control via gRPC.

> [!WARNING]
> pwright grants full browser control (navigation, JS execution, screenshots, cookies) to any client that can reach its gRPC port. **By default, gRPC binds to `127.0.0.1` only.** Use `--bind-all` with caution and always firewall both the CDP (9222) and gRPC (50051) ports. See [Security Considerations](getting-started.md#security-considerations).

## User Guides

- [Architecture](architecture.md) — Crate structure, layer design, concurrency model
- [Getting Started](getting-started.md) — What is CDP, Chrome setup, `.pwright` folder, troubleshooting
- [CLI Guide](cli-guide.md) — Full command reference, snapshot workflow, agent setup
- [gRPC API Reference](grpc-api.md) — All RPCs with request/response schemas
- [Codelabs](codelabs.md) — Usage examples with Rust, CLI, and gRPC

## Concepts

- [Pre-compiled Browser Skills for Agents](recipes-for-agents.md) -- Why recipes beat runtime exploration

## Script Runner & Recipes

- [Script Examples](../examples/scripts/) — Ready-to-use YAML automation scripts
- [Recipes](../examples/recipes/) — Pre-built recipes for search, monitoring, email, automation
- [Codelabs: Script Runner](codelabs.md#11-script-runner) — Tutorial with examples
- [Script Runner Design](knowledge/script-runner-design.md) — Full design with protobuf schema and phases
- [Recipes Design](knowledge/recipes-design.md) — Recipe specifications and design principles

## Agent Integration

- [Agent Skill](../skill/pwright/SKILL.md) — Skill definition for AI agent frameworks

## Network Capture

- [Network Capture Design](knowledge/network-capture-design.md) — Second CDP session listener architecture
- CLI commands: `network-listen`, `network-list`, `network-get`

## Site Explorations

- [Reddit Structure](exploration/reddit-structure.md) — `shreddit-post` web component attributes (tested 2026-03-21)
- [X.com Structure](exploration/x-structure.md) — `data-testid` tweet selectors (tested 2026-03-21)

## Planning

- [Improvements](knowledge/pwright-improvements.md) — Proposed improvements (P0-P3, prioritized)
- [Recipe Catalog](knowledge/recipe-catalog.md) — Full proposed recipe list (EN + CN sites)

## Reference

- [Playwright API Mapping](knowledge/playwright-api.md) — Full Playwright API compatibility matrix
- [API Parity](knowledge/api-parity.md) — gRPC vs CLI vs Playwright bridge coverage gaps
- [Feature Comparison](knowledge/feature-comparison.md) — Playwright vs PinchTab vs pwright side-by-side
- [Testing Strategy](knowledge/testing-strategy.md) — FakeCdpClient + Docker integration tests
- [Known Issues](known-issues.md) — Bug tracker and fixed issues
- [Bug Reports](bugs/index.md) — Detailed investigation reports for significant bugs
- [TODO](todo.md) — Planned features and roadmap
