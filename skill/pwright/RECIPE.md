# Writing pwright Recipes

Guide for agents creating new YAML recipes for pwright's script runner.

## When to Write a Recipe

Write a recipe when a browser task will be repeated. Recipes turn multi-step
browser interactions into a single deterministic command with structured JSON
output. No LLM reasoning needed at execution time.

## Recipe Structure

```yaml
name: "Human-readable name"
type: query            # query (read-only) or mutation (side effects)
params:
  url:
    type: string
    required: true
  max_items:
    type: integer
    default: "10"
scripts:
  extract_data: |
    (() => {
      // JS that returns structured data
      return JSON.stringify([...document.querySelectorAll('.item')].map(el => ({
        title: el.querySelector('h2')?.textContent?.trim(),
        link: el.querySelector('a')?.href,
      })));
    })()
steps:
  - goto: "{{ url }}"
    wait_for: ".item"
  - eval: { ref: extract_data, save_as: data }
  - output: { items: "{{ data }}" }
```

## Step Types

| Step | Syntax | Purpose |
|------|--------|---------|
| `goto` | `goto: "{{ url }}"` | Navigate to URL |
| `click` | `click: "selector"` | Click an element |
| `fill` | `fill: { selector: "input", value: "text" }` | Set input value |
| `press` | `press: "Enter"` | Press keyboard key |
| `extract` | `extract: { selector: "h1", field: text_content, save_as: title }` | Extract element data |
| `eval` | `eval: { ref: script_name, save_as: var }` | Run JS from scripts section |
| `output` | `output: { key: "{{ var }}" }` | Emit structured JSON |
| `wait` | `wait: 2000` | Sleep (ms) |

### Extract Fields

`text_content`, `inner_text`, `inner_html`, `input_value`, `is_visible`,
`is_checked`, `is_disabled`, `attribute:<name>` (e.g. `attribute:href`).

### Error Handling

```yaml
- click: ".optional-banner-dismiss"
  on_error: continue    # skip if element not found (default: fail)
```

## Patterns from Production Use

### 1. Prefer One Big eval Over Many extract Steps

A single JS evaluation that returns structured JSON is faster and more
reliable than multiple CDP round-trips:

```yaml
# GOOD: one eval, structured output
scripts:
  extract_all: |
    (() => {
      const rows = document.querySelectorAll('tr.data-row');
      return JSON.stringify([...rows].map(r => ({
        name: r.cells[0]?.textContent?.trim(),
        value: r.cells[1]?.textContent?.trim(),
      })));
    })()
steps:
  - eval: { ref: extract_all, save_as: rows }
  - output: { rows: "{{ rows }}" }

# BAD: N separate extract steps
steps:
  - extract: { selector: "tr:nth-child(1) td:nth-child(1)", field: text_content, save_as: name1 }
  - extract: { selector: "tr:nth-child(1) td:nth-child(2)", field: text_content, save_as: val1 }
  # ... repeated N times
```

### 2. SPA Hydration: Poll body.innerText, Not Selectors

Modern SPAs (Angular, React) load shell HTML immediately but render content
via JS. `document.readyState === 'complete'` fires before the framework
hydrates. Use `wait_for` with a selector that only appears after hydration,
or use a wait + eval polling loop:

```yaml
steps:
  - goto: "{{ url }}"
    wait_for: "[data-loaded='true']"   # framework sets this after hydration
```

If no reliable hydration selector exists, use a generous wait:

```yaml
steps:
  - goto: "{{ url }}"
  - wait: 5000    # give SPA time to hydrate
```

### 3. Text-Based Extraction Over Selector-Based

For sites with unstable DOM (frequent redesigns, A/B tests, shadow DOM),
extract via `document.body.innerText` and parse in the eval JS rather than
relying on CSS selectors that break:

```yaml
scripts:
  extract_by_marker: |
    (() => {
      const text = document.body?.innerText || '';
      const marker = 'Results:';
      const idx = text.indexOf(marker);
      if (idx < 0) return JSON.stringify({ error: 'marker not found' });
      const after = text.substring(idx + marker.length).trim();
      return JSON.stringify({ content: after.split('\n')[0] });
    })()
```

### 4. Response Stability: Wait for Streaming to Finish

When extracting from pages that stream content (chat UIs, live feeds),
a single read may capture partial data. Use a stability check -- poll
until N consecutive reads return identical content:

```yaml
scripts:
  wait_stable: |
    (async () => {
      let last = '', stable = 0;
      while (stable < 3) {
        await new Promise(r => setTimeout(r, 1000));
        const current = document.body?.innerText || '';
        if (current === last) stable++; else stable = 0;
        last = current;
      }
      return JSON.stringify({ text: last });
    })()
steps:
  - goto: "{{ url }}"
  - eval: { ref: wait_stable, save_as: content }
  - output: { content: "{{ content }}" }
```

### 5. Text Insertion for Rich Editors

Rich text editors (Quill, ProseMirror, TipTap) listen for DOM input events.
CDP's `Input.insertText` may bypass these events. Use `execCommand` first:

```yaml
scripts:
  insert_text: |
    (text) => {
      const el = document.activeElement;
      if (!document.execCommand('insertText', false, text)) {
        // Fallback: set value directly + dispatch input event
        el.value = text;
        el.dispatchEvent(new InputEvent('input', { bubbles: true }));
      }
    }
steps:
  - click: ".editor-input"
  - eval: { ref: insert_text, args: ["{{ prompt }}"] }
```

### 6. JSON Extraction from LLM Output

LLMs wrap JSON in markdown fences or add explanatory text. Extract
defensively with multiple strategies:

```yaml
scripts:
  extract_json: |
    (() => {
      const text = document.body?.innerText || '';
      // Try markdown fence
      const fenceMatch = text.match(/```(?:json)?\s*([\s\S]*?)```/);
      if (fenceMatch) return fenceMatch[1].trim();
      // Try first { ... }
      const braceMatch = text.match(/\{[\s\S]*\}/);
      if (braceMatch) return braceMatch[0];
      // Raw fallback
      return JSON.stringify({ raw: text });
    })()
```

## Validation

Always validate before running:

```bash
pwright script validate recipe.yaml --param url=https://example.com
```

Checks: required params supplied, template refs resolve, JS registry refs
exist, selectors non-empty.

## Running

```bash
# With inline params
pwright script run recipe.yaml --param url=https://example.com --param max=10

# With param file (for credentials)
pwright script run recipe.yaml --param-file secrets.yaml

# Output is JSONL (one JSON object per step + summary)
```

## Network Capture Workflow

For sites where the data comes from API calls (most modern SPAs),
use network capture to reverse-engineer the API instead of scraping the DOM:

```bash
# Terminal 1: start listener filtered to API calls
pwright network-listen --type XHR,Fetch --filter "/api/"

# Terminal 2: interact with the site
pwright open https://example.com
pwright snapshot
pwright fill e3 "search query"
pwright press Enter

# Terminal 1 shows:
# {"event":"request","reqid":"1001","method":"POST","url":".../api/search",...}
# {"event":"response","reqid":"1001","status":200,"mime":"application/json",...}

# Grab the response body
pwright network-get 1001
# {"results": [{"title": "Result 1", ...}]}

# Quick resource audit (no listener needed)
pwright network-list
```

Once you know the API shape, write a direct API call instead of browser automation:
the captured request shows the URL, method, headers, and POST body.

See `examples/recipes/network/` for complete shell scripts.

## Exploration-First Workflow

When creating a recipe for a new site:

1. Open the site: `pwright open https://target-site.com`
2. Snapshot: `pwright snapshot` -- see the accessibility tree
3. Test selectors: `pwright eval "document.querySelector('.data')?.textContent"`
4. Try network capture: `pwright network-listen --filter "/api/"` (in another terminal)
5. Interact with the page and observe API calls
6. Package the working selectors/JS/API calls into a YAML recipe
7. Validate: `pwright script validate recipe.yaml`

This is a one-time cost per site. Every execution after that is deterministic.
