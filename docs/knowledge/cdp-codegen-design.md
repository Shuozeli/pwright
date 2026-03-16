# Design: CDP Protocol Codegen Crate (`pwright-cdp-gen`)

## Problem

`pwright-cdp` hand-writes CDP domain wrappers in `domains/*.rs`. Each wrapper:
- Manually constructs `serde_json::json!({...})` for command params
- Manually extracts fields from `serde_json::Value` responses
- Has no compile-time guarantee that params/returns match the protocol spec
- Only covers ~44 of ~300+ commands across 9 of 55 domains

When adding new CDP commands, developers must read the protocol docs, write
the JSON construction by hand, and hope the field names are correct. This is
the same problem Swagger/OpenAPI codegen solves for REST APIs.

## Solution

Generate typed Rust structs and async methods from the CDP protocol JSON spec.
The CDP protocol has a well-defined JSON schema at:

- **browser_protocol.json** (49 domains, ~1.4MB): https://raw.githubusercontent.com/ChromeDevTools/devtools-protocol/master/json/browser_protocol.json
- **js_protocol.json** (6 domains, ~180KB): https://raw.githubusercontent.com/ChromeDevTools/devtools-protocol/master/json/js_protocol.json

Canonical source: https://github.com/ChromeDevTools/devtools-protocol/tree/master/json

Or fetch live from a running Chrome: `http://localhost:9222/json/protocol`

## Protocol Schema Structure

```
protocol.json
  version: { major: "1", minor: "3" }
  domains: [
    {
      domain: "Page",
      dependencies: ["DOM", "Network"],
      types: [
        { id: "FrameId", type: "string" },
        { id: "TransitionType", type: "string", enum: ["link", "typed", ...] },
        { id: "Frame", type: "object", properties: [...] }
      ],
      commands: [
        {
          name: "navigate",
          parameters: [
            { name: "url", type: "string" },
            { name: "referrer", type: "string", optional: true },
            { name: "transitionType", $ref: "TransitionType", optional: true }
          ],
          returns: [
            { name: "frameId", $ref: "FrameId" },
            { name: "errorText", type: "string", optional: true }
          ]
        }
      ],
      events: [
        { name: "loadEventFired", parameters: [...] }
      ]
    }
  ]
```

## CDP Coverage Today

pwright uses 9 of 55 domains, ~44 of ~300+ available commands:

| Domain | Commands Used | Total Available | Coverage |
|--------|--------------|-----------------|----------|
| Accessibility | 2 | 8 | 25% |
| Browser | 1 | 20 | 5% |
| DOM | 13 | 53 | 25% |
| Fetch | 4 | 9 | 44% |
| Input | 4 | 13 | 31% |
| Network | 5 | 40 | 13% |
| Page | 10 | 61 | 16% |
| Runtime | 4 | 23 | 17% |
| Target | 5 | 19 | 26% |

Most responses are parsed as `serde_json::Value` with manual field extraction.
Only a handful have typed structs: `TargetInfo`, `Cookie`, `RawAXNode`.

## Proposed Crate: `pwright-cdp-gen`

### What It Generates

For each CDP domain, generate:

1. **Type definitions** - Rust structs/enums for every CDP type
2. **Command param structs** - Typed request params with builder pattern
3. **Command return structs** - Typed response structs
4. **Event structs** - Typed event payload structs
5. **Domain trait or impl** - Async methods on `CdpSession`

### Example Output

From `Page.navigate` in the protocol JSON:

```rust
// Generated type aliases
pub type FrameId = String;

// Generated enum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TransitionType {
    Link, Typed, AddressBar, AutoBookmark, AutoSubframe,
    ManualSubframe, Generated, AutoToplevel, FormSubmit,
    Reload, Keyword, KeywordGenerated, Other,
}

// Generated command params
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NavigateParams {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referrer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition_type: Option<TransitionType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<FrameId>,
}

// Generated return type
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigateResult {
    pub frame_id: FrameId,
    pub loader_id: Option<String>,
    pub error_text: Option<String>,
}

// Generated method
impl CdpSession {
    pub async fn page_navigate(&self, params: NavigateParams) -> Result<NavigateResult> {
        let value = serde_json::to_value(&params)?;
        let result = self.send("Page.navigate", value).await?;
        Ok(serde_json::from_value(result)?)
    }
}
```

### Implementation (completed)

The codegen is a standalone binary (`cdp-gen`) that reads protocol JSON and
writes `.rs` files to `pwright-cdp/src/generated/`. No build-time dependency
-- generated code is committed to git. The codegen runs manually when updating
the protocol.

**Protocol sources:**
- Live Chrome: `curl -s http://HOST:9222/json/protocol > proto/cdp/chrome_protocol.json`
- Vendored GitHub: `proto/cdp/browser_protocol.json` + `js_protocol.json`
- HTTP endpoint aliases: `proto/cdp/http_endpoints.json` (data-driven, not hardcoded)

**Domain selection:** Hardcoded in `codegen.rs::DOMAINS` (9 domains).

### Crate Structure

```
crates/pwright-cdp-gen/
  Cargo.toml
  proto/cdp/
    chrome_protocol.json    # Live protocol from Chrome /json/protocol
    browser_protocol.json   # Vendored GitHub spec (fallback)
    js_protocol.json        # Vendored GitHub spec (fallback)
    http_endpoints.json     # Field aliases for Chrome HTTP debug API
  src/
    main.rs           # CLI entrypoint, protocol loading, domain iteration
    codegen.rs        # Rust code generator (types, enums, structs, params, returns, events)
    protocol.rs       # Protocol JSON schema types (Domain, Command, Type, Property, etc.)
```

### Relationship to `pwright-cdp`

```
pwright-cdp-gen (standalone binary, dev tool)
  reads:  proto/cdp/chrome_protocol.json + http_endpoints.json
  writes: pwright-cdp/src/generated/*.rs (committed to git)

pwright-cdp (library, no dependency on cdp-gen)
  src/generated/     # Generated types, enums, params, returns, events
  src/domains/       # Hand-written methods using generated types
```

### Resolved Questions

1. **Standalone binary** -- not build.rs, not a dependency. Generated code
   is committed. Same pattern as protobuf codegen.

2. **Live Chrome protocol preferred** -- `chrome_protocol.json` from
   `/json/protocol` matches the exact Chrome version. Vendored GitHub
   spec available as `--vendored` fallback.

3. **Cross-domain refs** -- resolved via `super::network::LoaderId`.
   Refs to non-generated domains fall back to `serde_json::Value`.

4. **Backward compatibility** -- hand-written methods in `domains/*.rs`
   keep the same public API. They construct generated `*Params` structs
   internally and deserialize into generated `*Returns` structs.

5. **HTTP endpoint aliases** -- data-driven via `http_endpoints.json`.
   The codegen reads the aliases and emits `#[serde(alias = "...")]`
   on the appropriate struct fields. No hardcoded serde attributes.

6. **CDP `binary` type** -- mapped to `String` (base64-encoded). Present
   in live Chrome protocol but not in vendored spec.

7. **Recursive types** -- `Node` and `StackTrace` self-references wrapped
   in `Box<>` to break infinite size.

8. **Optional field serialization** -- type structs emit
   `#[serde(skip_serializing_if = "Option::is_none")]` to prevent sending
   `null` values that Chrome rejects as invalid parameters.

## Prior Art

- **chromiumoxide** (Rust) - Full CDP codegen from protocol JSON. Heavy (~200 files generated).
- **rust-cdp** (Rust) - Older codegen approach, less maintained.
- **playwright** (TS) - Generates protocol types from PDL files.
- **chromedp** (Go) - Generates Go types from protocol JSON.
- **puppeteer** (TS) - Uses protocol types but not full codegen.

## Remaining Phases

| Phase | Scope | Status |
|-------|-------|--------|
| 1 | Parser + codegen for types/enums | Done |
| 2 | Codegen for command params + return types | Done |
| 3 | Migrate pwright-cdp domains to use generated types | Done |
| 4 | Codegen for event types | Done (structs generated, dispatcher not yet) |
| 5 | Add new domains (Emulation, Console, Log) via config | Not started |
| 6 | Codegen unit tests (serialization round-trips) | Not started |
