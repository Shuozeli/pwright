# Script Runner Design

## Overview

A declarative script runner for pwright that takes a YAML script with parameters,
executes browser automation steps sequentially (with control flow), and produces
structured JSONL output with full observability.

## Architecture

```
pwright-script/          New crate: script parser, validator, executor
  proto/script.proto     Protobuf schema for script input/output
  src/
    lib.rs
    parser.rs            YAML -> Script AST
    validator.rs         Validate script against proto schema
    executor.rs          Execute script steps against pwright-bridge
    output.rs            JSONL streaming output

pwright-cli/
  src/main.rs            `pwright run script.yaml --param key=val`
```

### Crate Dependencies

```
pwright-script
  depends on: pwright-bridge, pwright-cdp, serde_yaml, prost
  used by: pwright-cli

pwright-cli
  depends on: pwright-script (new)
```

## CLI Interface

```bash
# Basic execution
pwright run crawl.yaml

# With parameters
pwright run crawl.yaml --param url=https://example.com --param max_pages=10

# Parameters from file (for credentials etc.)
pwright run crawl.yaml --param-file secrets.yaml

# Streaming output to file
pwright run crawl.yaml --output results.jsonl

# Debug mode (verbose step-by-step with CDP details)
pwright run crawl.yaml --debug

# Validate without executing
pwright run crawl.yaml --validate

# Dry run (parse + validate + show plan)
pwright run crawl.yaml --dry-run
```

### Param File Format

```yaml
# secrets.yaml
email: "user@example.com"
password: "secret123"
api_key: "sk-..."
```

Param file values are merged with `--param` flags. `--param` takes precedence.

## Script Format (YAML)

### JS Script Registry

JavaScript code is defined centrally in a `scripts` section, not inline in
steps. Steps reference scripts by name via `ref`. This keeps JS maintainable,
testable, and avoids YAML escaping issues.

```yaml
scripts:
  extract_prices: |
    [...document.querySelectorAll('.price')]
      .map(el => ({
        text: el.textContent.trim(),
        value: parseFloat(el.dataset.value)
      }))

  scroll_to_bottom: |
    window.scrollTo(0, document.body.scrollHeight);
    await new Promise(r => setTimeout(r, 500));

  get_page_metadata: |
    ({
      title: document.title,
      url: window.location.href,
      canonical: document.querySelector('link[rel=canonical]')?.href
    })

steps:
  - eval: { ref: scroll_to_bottom }
  - eval: { ref: extract_prices, save_as: prices }
  - eval: { ref: get_page_metadata, save_as: meta }
```

Scripts can also accept arguments via the `args` field, which maps to
`Runtime.callFunctionOn` arguments:

```yaml
scripts:
  click_nth_item: |
    function(index) {
      document.querySelectorAll('.item')[index].click();
    }

steps:
  - eval: { ref: click_nth_item, args: [3] }
```

#### External JS Files

For large scripts, reference external `.js` files. The parser inlines
them into the `scripts` registry at parse time:

```yaml
scripts_dir: ./js/          # relative to script YAML location
scripts_files:
  - extract_prices           # loads ./js/extract_prices.js
  - complex_scraper          # loads ./js/complex_scraper.js

steps:
  - eval: { ref: extract_prices, save_as: prices }
```

### Minimal Example

```yaml
name: "Extract page title"
params:
  url:
    type: string
    required: true

steps:
  - goto: "{{ url }}"
  - extract:
      selector: "h1"
      field: text_content
      output: title
```

### Full Example (Crawler)

```yaml
name: "News article scraper"
description: "Scrape article links and their content"
version: 1

params:
  url:
    type: string
    required: true
    description: "Starting URL"
  max_articles:
    type: integer
    default: 10
  wait_timeout_ms:
    type: integer
    default: 5000

config:
  default_timeout_ms: "{{ wait_timeout_ms }}"

scripts:
  extract_article_links: |
    [...document.querySelectorAll('article a')]
      .map(a => a.href)
      .filter(href => href.startsWith('http'))

  clean_body_text: |
    function(selector) {
      const el = document.querySelector(selector);
      if (!el) return '';
      // Remove ads, nav elements
      el.querySelectorAll('.ad, nav, .sidebar').forEach(e => e.remove());
      return el.innerText.trim();
    }

steps:
  # Navigate to listing page
  - goto: "{{ url }}"
    wait_for: "article a"

  # Extract article links via JS (handles dynamic SPAs better)
  - eval:
      ref: extract_article_links
      save_as: links

  # Visit each article
  - for_each:
      items: "{{ links }}"
      as: link
      on_error: continue    # skip failed articles
      steps:
        - goto: "{{ link }}"
          wait_for: ".article-body"

        - extract:
            selector: "h1"
            field: text_content
            save_as: title

        - eval:
            ref: clean_body_text
            args: [".article-body"]
            save_as: body

        - output:
            title: "{{ title }}"
            url: "{{ link }}"
            body: "{{ body }}"

  # Final assertion
  - assert:
      expr: "outputs | length >= 1"
      message: "Expected at least 1 article"
```

### Paginated Scrape Example

```yaml
name: "Search results scraper"
params:
  query: { type: string, required: true }

config:
  default_timeout_ms: 10000

steps:
  - goto: "https://example.com/search?q={{ query }}"
    wait_for: ".search-results"

  - paginate:
      next_selector: "a.next-page"
      max_pages: 20
      wait_for: ".search-results"
      steps:
        - extract_all:
            selector: ".result-item"
            return_type: elements
            save_as: items

        - for_each:
            items: "{{ items }}"
            as: item
            steps:
              - extract:
                  element: "{{ item }}"
                  selector: "h3"
                  field: text_content
                  save_as: title
              - extract:
                  element: "{{ item }}"
                  selector: "a"
                  field: "attribute:href"
                  save_as: url
              - output:
                  title: "{{ title }}"
                  url: "{{ url }}"
```

### Login + Scrape Example

```yaml
name: "Authenticated scraper"
params:
  login_url: { type: string, required: true }
  email: { type: string, required: true }
  password: { type: string, required: true }
  target_url: { type: string, required: true }

steps:
  - goto: "{{ login_url }}"
    wait_for: "#email"

  - fill:
      selector: "#email"
      value: "{{ email }}"

  - fill:
      selector: "#password"
      value: "{{ password }}"

  - click: "#submit"
    wait_for: ".dashboard"
    timeout_ms: 10000

  - goto: "{{ target_url }}"
    wait_for: ".data-table"

  - extract_all:
      selector: ".data-table tr"
      fields:
        name: { selector: "td:nth-child(1)", field: text_content }
        value: { selector: "td:nth-child(2)", field: text_content }
      output: rows
```

## Step Primitives

### Navigation

| Step | Description | Fields |
|------|-------------|--------|
| `goto` | Navigate to URL | `url`, `wait_for?`, `timeout_ms?` |
| `reload` | Reload page | `wait_for?`, `timeout_ms?` |
| `go_back` | Browser back | |
| `go_forward` | Browser forward | |

### Actions

| Step | Description | Fields |
|------|-------------|--------|
| `click` | Click element | `selector`, `wait_for?` |
| `fill` | Fill input | `selector`, `value` |
| `type` | Type text | `selector`, `text` |
| `press` | Press key | `key` |
| `check` | Check checkbox | `selector` |
| `uncheck` | Uncheck checkbox | `selector` |
| `select` | Select option | `selector`, `value` |
| `scroll` | Scroll to element | `selector` |

### Extraction

| Step | Description | Fields |
|------|-------------|--------|
| `extract` | Extract single value | `selector`, `field`, `save_as?`, `output?` |
| `extract_all` | Extract list of values | `selector`, `field`/`fields`, `save_as?`, `limit?`, `return_type?` |
| `eval` | Evaluate JS | `expression`, `save_as?` |
| `snapshot` | Take accessibility snapshot | `filter?`, `save_as?` |
| `screenshot` | Take screenshot | `filename?`, `format?` |

`field` values: `text_content`, `inner_text`, `inner_html`, `attribute:<name>`,
`input_value`, `is_visible`, `is_checked`, `is_disabled`

`return_type` for `extract_all`:
- `values` (default): returns a flat list of extracted strings
- `elements`: returns DOM element handles that can be used with `for_each`
  for scoped sub-queries within each element

```yaml
# Values mode (default) - returns ["link1", "link2", ...]
- extract_all:
    selector: "article a"
    attribute: href
    return_type: values
    save_as: links

# Elements mode - returns element handles for scoped extraction
- extract_all:
    selector: "article"
    return_type: elements
    save_as: articles

- for_each:
    items: "{{ articles }}"
    as: article
    steps:
      - extract:
          element: "{{ article }}"      # scoped to this element
          selector: "h2"
          field: text_content
          save_as: title
```

### Control Flow

| Step | Description | Fields |
|------|-------------|--------|
| `for_each` | Loop over items | `items`, `as`, `steps`, `on_error?` |
| `paginate` | Click through pages | `next_selector`, `max_pages?`, `steps` |
| `retry` | Retry on failure | `times`, `delay_ms`, `steps` |
| `if` | Conditional | `condition`, `then`, `else?` |
| `wait` | Wait for condition | `selector`, `state?`, `timeout_ms?` |

#### Pagination

The `paginate` step handles the common "click next, extract, repeat" pattern:

```yaml
- paginate:
    next_selector: "a.next-page"     # click this to go to next page
    max_pages: 50                     # safety limit (default: 100)
    wait_for: ".results"              # wait after each page transition
    steps:
      - extract_all:
          selector: ".result-item h3"
          field: text_content
          output: items
```

Execution flow:
1. Execute `steps` on current page
2. Check if `next_selector` exists
3. If yes: click it, wait for `wait_for`, go to step 1
4. If no: stop (all pages scraped)

### Output

| Step | Description | Fields |
|------|-------------|--------|
| `output` | Emit a result row | Key-value map with template values |
| `assert` | Assert condition | `expr`, `message?` |
| `log` | Debug log | `message` |

### Error Handling

Each step supports:
```yaml
- goto: "{{ url }}"
  on_error: fail       # fail (default) | continue | retry
  retry_times: 3
  retry_delay_ms: 1000
```

## Variable System

### Template Syntax

Variables use `{{ name }}` syntax (Jinja2-like):

```yaml
- goto: "{{ url }}"          # param reference
- fill:
    selector: "#q"
    value: "{{ query }}"     # param reference
- extract:
    selector: "h1"
    save_as: title           # creates variable
- output:
    title: "{{ title }}"     # variable reference
```

### Variable Scopes

1. **Params**: defined in `params:`, passed via `--param key=val`
2. **Step outputs**: created by `save_as` in extract/eval/snapshot
3. **Loop variables**: `{{ item }}` / `{{ index }}` in `for_each`
4. **Built-in**: `{{ __step_index }}`, `{{ __timestamp }}`

## Protobuf Schema

The input script structure and output format are defined in protobuf
for type safety and cross-language compatibility.

```protobuf
syntax = "proto3";
package pwright.script.v1;

// ── Script Input ──

message Script {
  string name = 1;
  string description = 2;
  int32 version = 3;
  map<string, ParamDef> params = 4;
  repeated Step steps = 5;
  ScriptConfig config = 6;
  map<string, JsFunction> scripts = 7;    // central JS registry
}

// A named JavaScript function in the central registry.
message JsFunction {
  string body = 1;               // JS source code
  string description = 2;        // human-readable description
  bool is_async = 3;             // use awaitPromise when executing
}

message ScriptConfig {
  int32 default_timeout_ms = 1;   // inherited by all steps (default: 30000)
  ErrorPolicy default_on_error = 2;
}

message ParamDef {
  string type = 1;        // string, integer, boolean
  bool required = 2;
  string default = 3;
  string description = 4;
}

message Step {
  oneof kind {
    GotoStep goto = 1;
    ClickStep click = 2;
    FillStep fill = 3;
    TypeStep type_text = 4;
    PressStep press = 5;
    ExtractStep extract = 6;
    ExtractAllStep extract_all = 7;
    EvalStep eval = 8;
    ForEachStep for_each = 9;
    RetryStep retry = 10;
    OutputStep output = 11;
    AssertStep assert = 12;
    SnapshotStep snapshot = 13;
    ScreenshotStep screenshot = 14;
    WaitStep wait = 15;
    CheckStep check = 16;
    SelectStep select = 17;
    LogStep log = 18;
    ReloadStep reload = 19;
    PaginateStep paginate = 20;
  }
  ErrorPolicy on_error = 30;
  int32 retry_times = 31;
  int32 retry_delay_ms = 32;
}

enum ErrorPolicy {
  FAIL = 0;
  CONTINUE = 1;
  RETRY = 2;
}

message GotoStep {
  string url = 1;
  string wait_for = 2;
  int32 timeout_ms = 3;
}

message ClickStep {
  string selector = 1;
  string wait_for = 2;
}

message FillStep {
  string selector = 1;
  string value = 2;
}

message TypeStep {
  string selector = 1;
  string text = 2;
}

message PressStep {
  string key = 1;
}

message ExtractStep {
  string selector = 1;
  string field = 2;         // text_content, inner_text, attribute:<name>, etc.
  string save_as = 3;
  bool output = 4;          // also emit as output row
}

message ExtractAllStep {
  string selector = 1;
  string field = 2;
  map<string, FieldDef> fields = 3;   // for structured extraction
  string save_as = 4;
  int32 limit = 5;
  ReturnType return_type = 6;
}

enum ReturnType {
  VALUES = 0;              // flat list of extracted strings (default)
  ELEMENTS = 1;            // DOM element handles for scoped sub-queries
}

message FieldDef {
  string selector = 1;
  string field = 2;
}

message EvalStep {
  oneof source {
    string expression = 1;     // inline JS (discouraged for complex code)
    string ref = 4;            // reference to scripts registry entry
  }
  string save_as = 2;
  repeated google.protobuf.Value args = 3;   // arguments passed to the function
  // When using ref, the script body is wrapped in a function call:
  //   ref: "extract_prices" -> Runtime.callFunctionOn(body, args)
  // When using expression, it's evaluated directly:
  //   expression: "1+1" -> Runtime.evaluate(expression)
}

message ForEachStep {
  string items = 1;         // variable name
  string as = 2;            // loop variable name
  repeated Step steps = 3;
  ErrorPolicy on_error = 4;
}

message RetryStep {
  int32 times = 1;
  int32 delay_ms = 2;
  repeated Step steps = 3;
}

message OutputStep {
  map<string, string> fields = 1;
}

message AssertStep {
  string expr = 1;
  string message = 2;
}

message SnapshotStep {
  string filter = 1;
  string save_as = 2;
}

message ScreenshotStep {
  string filename = 1;
  string format = 2;
}

message WaitStep {
  string selector = 1;
  string state = 2;        // attached, visible, hidden, detached
  int32 timeout_ms = 3;
}

message CheckStep {
  string selector = 1;
}

message SelectStep {
  string selector = 1;
  string value = 2;
}

message LogStep {
  string message = 1;
}

message ReloadStep {
  string wait_for = 1;
  int32 timeout_ms = 2;
}

message PaginateStep {
  string next_selector = 1;    // selector for "next page" button/link
  int32 max_pages = 2;         // safety limit (default: 100)
  string wait_for = 3;         // wait for this selector after each page transition
  int32 timeout_ms = 4;
  repeated Step steps = 5;     // steps to execute on each page
}

// ── Script Output ──

message StepResult {
  string timestamp = 1;
  int32 step_index = 2;
  string step_type = 3;      // "goto", "click", "extract", etc.
  string status = 4;          // "ok", "error", "skipped"
  int32 duration_ms = 5;
  map<string, string> details = 6;   // step-specific output
  string error = 7;           // error message if status == "error"
}

message ScriptResult {
  string timestamp = 1;
  string name = 2;
  string status = 3;          // "ok", "error"
  int32 total_steps = 4;
  int32 succeeded = 5;
  int32 failed = 6;
  int32 skipped = 7;
  int32 duration_ms = 8;
  repeated OutputRow outputs = 9;
  string error = 10;
}

message OutputRow {
  map<string, string> fields = 1;
}
```

## Output Format

### JSONL Streaming (default)

Each completed step emits one JSON line to stdout (or `--output` file):

```jsonl
{"ts":"...","step":0,"type":"goto","status":"ok","ms":1200,"url":"https://example.com"}
{"ts":"...","step":1,"type":"extract","status":"ok","ms":5,"save_as":"title","value":"Hello"}
{"ts":"...","step":2,"type":"for_each","status":"ok","ms":3000,"iterations":5}
```

### Debug Mode (`--debug`)

Adds verbose info: CDP commands sent, selector resolution, variable state:

```jsonl
{"ts":"...","step":0,"type":"goto","status":"ok","ms":1200,"debug":{"cdp_commands":["Page.navigate","Runtime.evaluate"],"wait_polls":3}}
```

### Final Summary (last line)

```jsonl
{"summary":true,"status":"ok","name":"News scraper","steps":12,"ok":12,"failed":0,"ms":5000,"outputs":[{"title":"...","url":"..."}]}
```

### Error Output

```jsonl
{"ts":"...","step":3,"type":"click","status":"error","error":"Element not found for selector: .nonexistent","ms":5000}
{"summary":true,"status":"error","name":"News scraper","steps":4,"ok":3,"failed":1,"ms":6200,"error":"Step 3 failed: Element not found"}
```

## Validation

The `pwright-script` crate includes a validator that checks:

1. **Schema validity**: YAML parses into valid Script proto message
2. **Param completeness**: all `required: true` params have values
3. **Template references**: all `{{ var }}` references point to defined params or prior `save_as`
4. **Selector format**: selectors are non-empty strings
5. **Step-specific rules**: e.g. `for_each` has `items` and `steps`
6. **JS registry references**: all `eval.ref` values exist in `scripts` map
7. **JS syntax check**: basic validation that script bodies are parseable
   (bracket matching, no obvious syntax errors)

```bash
pwright run script.yaml --validate
# OK: 12 steps, 3 params (url, max_pages, timeout)
# WARNING: param 'timeout' has no default and is not required

pwright run script.yaml --validate --param url=https://example.com
# OK: all params resolved, 12 steps valid
```

## Implementation Phases

### Phase 1: Crate scaffold + protobuf + parser + basic execution

**Goal:** `pwright run script.yaml --param url=... ` works end-to-end
for a flat script with basic steps.

#### Deliverables

1. **`pwright-script` crate** with Cargo.toml, deps: pwright-bridge, prost,
   serde_yaml, serde_json
2. **Protobuf schema** (`proto/pwright/script/v1/script.proto`)
   - Messages: `Script`, `ScriptConfig`, `ParamDef`, `Step`, `JsFunction`
   - Step messages: `GotoStep`, `ClickStep`, `FillStep`, `ExtractStep`,
     `EvalStep`, `OutputStep`, `PressStep`
   - Output messages: `StepResult`, `ScriptResult`, `OutputRow`
   - `prost-build` codegen in `build.rs`
3. **YAML parser** (`parser.rs`)
   - Parse YAML into `Script` proto message
   - Template substitution for `{{ param }}` references
   - `--param key=val` and `--param-file file.yaml` param loading
4. **Validator** (`validator.rs`)
   - Schema validation: required fields present, correct types
   - Param validation: all required params supplied
   - Template validation: all `{{ ref }}` resolve to params or prior `save_as`
   - JS registry validation: all `eval.ref` exist in `scripts` map
5. **Executor** (`executor.rs`)
   - Sequential step execution against `pwright-bridge`
   - `save_as` variable storage (HashMap<String, Value>)
   - JSONL output per step (StepResult as JSON)
   - Final ScriptResult summary
6. **CLI integration** (`pwright run` subcommand)
   - `pwright run script.yaml --param key=val`
   - `pwright run script.yaml --validate` (validate only)
   - `--output file.jsonl` (write results to file)

#### Steps supported in Phase 1

| Step | Proto message |
|------|--------------|
| `goto` | GotoStep (url, wait_for, timeout_ms) |
| `click` | ClickStep (selector) |
| `fill` | FillStep (selector, value) |
| `press` | PressStep (key) |
| `extract` | ExtractStep (selector, field, save_as) |
| `eval` | EvalStep (expression or ref, save_as, args) |
| `output` | OutputStep (fields map) |

#### Test plan

- Unit: YAML -> proto parsing for each step type
- Unit: template substitution with params
- Unit: validator catches missing params, invalid refs, bad selectors
- Unit: JS registry ref resolution
- Integration (FakeCdpClient): execute a 5-step script, verify JSONL output
- Integration (Docker): execute against real Chrome with test server
- E2E: `pwright run examples/hello.yaml --param url=http://... --validate`

#### Acceptance criteria

- [ ] `pwright run hello.yaml --param url=http://example.com` navigates,
      extracts title, outputs JSONL
- [ ] `pwright run hello.yaml --validate` exits 0 with valid script, exits 1 with
      clear error for invalid
- [ ] `--param-file` loads params from YAML file
- [ ] JS scripts in `scripts:` section are referenced by `eval.ref`
- [ ] StepResult JSONL includes timestamp, step_index, type, status, duration_ms
- [ ] ScriptResult summary is the last JSONL line

---

### Phase 2: Variables + control flow + error handling

**Goal:** Scripts can loop, branch, retry, and handle errors gracefully.

#### Deliverables

1. **Variable system** (`variables.rs`)
   - Scope management: params -> step outputs -> loop vars
   - `save_as` creates variables from extract/eval results
   - `{{ var }}` template resolution from variable stack
   - `{{ item }}` and `{{ index }}` in loop context
2. **`for_each` step**
   - Iterate over array variable (from `extract_all` or `eval`)
   - `as` names the loop variable
   - `on_error: continue` skips failed iterations
   - Nested `for_each` with scoped variables
3. **`retry` step**
   - Retry inner steps N times with delay
   - Exponential backoff option
4. **`if` step**
   - Condition evaluation (variable truthiness, comparison)
   - `then` and `else` branches
5. **Error policies**
   - Per-step `on_error: fail | continue | retry`
   - `retry_times` and `retry_delay_ms` per step
   - `ScriptConfig.default_on_error`
6. **`extract_all` step**
   - `return_type: values | elements`
   - `fields` map for structured extraction
   - `limit` cap

#### Steps added

| Step | Proto message |
|------|--------------|
| `for_each` | ForEachStep (items, as, steps, on_error) |
| `retry` | RetryStep (times, delay_ms, steps) |
| `if` | IfStep (condition, then, else) |
| `extract_all` | ExtractAllStep (selector, field, fields, save_as, limit, return_type) |

#### Test plan

- Unit: variable scoping (param shadows, loop scope cleanup)
- Unit: for_each iteration with save_as in inner steps
- Unit: retry with simulated failures
- Unit: if/else branching
- Unit: error policy enforcement (continue vs fail)
- Integration (FakeCdpClient): crawler script with for_each + extract_all
- Integration (Docker): scrape test server listing page with for_each

#### Acceptance criteria

- [ ] `for_each` iterates over extracted list, each iteration has scoped vars
- [ ] `on_error: continue` skips failed iterations, reports them in JSONL
- [ ] `retry` re-executes inner steps on failure with delay
- [ ] `extract_all` returns values or element handles per `return_type`
- [ ] Nested `for_each` works with proper variable scoping
- [ ] ScriptResult summary counts succeeded/failed/skipped correctly

---

### Phase 3: Pagination + advanced steps + observability

**Goal:** Production-ready for real crawler workloads.

#### Deliverables

1. **`paginate` step**
   - Click next_selector, wait, execute inner steps, repeat
   - Stop when next_selector disappears or max_pages reached
   - Page counter in JSONL output
2. **Additional action steps**
   - `check`, `uncheck`, `select` (checkbox/dropdown)
   - `scroll` (scroll element into view)
   - `type` (character-by-character typing)
   - `wait` with WaitState (attached/visible/hidden/detached)
3. **Content steps**
   - `snapshot` (accessibility tree)
   - `screenshot` (save to file)
   - `log` (debug message in JSONL)
   - `assert` (fail script on condition)
4. **Observability**
   - `--debug` mode: CDP commands, selector resolution, variable state
   - `--dry-run` mode: parse + validate + show execution plan
   - Step duration tracking with slow-step warnings
   - Error context: which page, which selector, what variable state

#### Steps added

| Step | Proto message |
|------|--------------|
| `paginate` | PaginateStep (next_selector, max_pages, wait_for, steps) |
| `check` | CheckStep (selector) |
| `select` | SelectStep (selector, value) |
| `wait` | WaitStep (selector, state, timeout_ms) |
| `snapshot` | SnapshotStep (filter, save_as) |
| `screenshot` | ScreenshotStep (filename, format) |
| `log` | LogStep (message) |
| `assert` | AssertStep (expr, message) |
| `reload` | ReloadStep (wait_for, timeout_ms) |

#### Test plan

- Unit: paginate with mock 3-page site
- Unit: assert pass/fail
- Unit: debug output includes CDP details
- Integration (Docker): paginate through test server multi-page results
- Integration (Docker): screenshot saved to file
- E2E: real crawler script against test server with pagination

#### Acceptance criteria

- [ ] `paginate` scrapes N pages, stops at missing next_selector
- [ ] `--debug` JSONL includes CDP command list per step
- [ ] `--dry-run` shows step plan without executing
- [ ] `screenshot` writes file to disk
- [ ] `assert` failure stops script with clear error message
- [ ] JSONL output is valid even on script failure (summary always emitted)

---

### Phase 4: External JS files + param files + polish

**Goal:** Production hardening for team use.

#### Deliverables

1. **External JS loading**
   - `scripts_dir` + `scripts_files` in YAML
   - Parser loads `.js` files and inlines into `JsFunction` messages
   - File-not-found errors at parse time
2. **Param file improvements**
   - `--param-file secrets.yaml` merges with `--param`
   - Environment variable substitution: `{{ $ENV.API_KEY }}`
   - Param validation: type checking (string vs integer vs boolean)
3. **Script composition**
   - `include` step: import steps from another YAML file
   - Shared `scripts` across included files
4. **Error reporting**
   - Source location in errors (line:column in YAML)
   - Suggestion messages ("did you mean X?")
   - `--format pretty` for human-readable output (vs JSONL)
5. **Performance**
   - Connection reuse across goto steps (single tab)
   - Parallel `for_each` option (multiple tabs)

#### Test plan

- Unit: external JS file loading
- Unit: env var substitution
- Unit: include step merging
- Integration: script with external JS against Docker Chrome
- E2E: crawler with param-file for credentials

---

### Phase 5: DSL (if YAML becomes unwieldy)

> **TODO: Evaluate DSL migration when:**
> - Nested `for_each` / `if` blocks exceed 3 levels of YAML indentation
> - Template expressions need arithmetic or string operations
> - Users request custom functions or macros
> - The YAML format becomes harder to read than equivalent code
>
> **Candidate DSL syntax (strawman):**
> ```
> script "News scraper" {
>   param url: string
>   param max_pages: int = 10
>
>   goto $url wait "article a"
>   let links = extract_all "article a" @href limit $max_pages
>
>   for link in $links {
>     goto $link wait ".article-body"
>     let title = extract "h1" text
>     let body = extract ".article-body" text
>     emit { title: $title, url: $link, body: $body }
>   }
>
>   assert count(outputs) >= 1 "Expected articles"
> }
> ```
>
> The DSL should be a thin syntax sugar over the same AST used by the
> YAML format. Both parse into the same `Script` protobuf message.

---

### Phase summary

| Phase | Scope | Steps | Key feature |
|-------|-------|-------|-------------|
| 1 | Scaffold | 7 | Basic end-to-end execution |
| 2 | Control flow | 4 | for_each, retry, if, variables |
| 3 | Production | 9 | paginate, debug, screenshots |
| 4 | Polish | - | External JS, env vars, includes |
| 5 | DSL | - | Custom syntax (if needed) |

## Design Decisions (Resolved)

1. **`extract_all` return type**: Supports both via `return_type` enum.
   `VALUES` (default) returns flat string list. `ELEMENTS` returns DOM
   handles for scoped sub-queries in `for_each`.

2. **Pagination**: Dedicated `paginate` step that clicks a "next" selector
   and repeats inner steps until the next button disappears or `max_pages`
   is reached. Simpler than a general `while` loop for the 90% case.

3. **Protobuf is source of truth**: The protobuf `Script` message is the
   canonical schema. YAML is a user-facing serialization format that maps
   to the protobuf. The validator checks YAML against the proto schema.

4. **Timeout inheritance**: `ScriptConfig.default_timeout_ms` applies to
   all steps that don't specify their own `timeout_ms`. Default: 30000ms.
   Steps can override per-step.

5. **Param files**: `--param-file secrets.yaml` loads key-value pairs.
   Merged with `--param` flags (flags take precedence). Useful for
   credentials that shouldn't be in command history.
