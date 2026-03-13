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

## Agent Integration

- [Agent Skill](../skill/pwright/SKILL.md) — Skill definition for AI agent frameworks (comparable to playwright-cli and pinchtab skills)

## Knowledge Base

Internal reference docs for contributors and developers:

- [CDP Protocol Notes](knowledge/cdp-protocol.md) — How pwright maps CDP domains to high-level operations
- [Playwright API Mapping](knowledge/playwright-api.md) — Full Playwright API compatibility matrix
- [API Parity](knowledge/api-parity.md) — gRPC vs CLI vs Playwright bridge coverage gaps
- [Feature Comparison](knowledge/feature-comparison.md) — Playwright vs PinchTab vs pwright side-by-side
- [Implementation Phases](knowledge/phases.md) — Phase 0–4 rollout plan with deliverables
