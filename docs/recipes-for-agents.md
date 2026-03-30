# Pre-compiled Browser Skills for AI Agents

## The Problem: Agents Redo Work They've Already Done

When an LLM agent needs to extract data from a web page, it typically:

1. Navigates to the page
2. Takes a snapshot (screenshot or accessibility tree)
3. Sends the snapshot to the LLM (~5-50KB depending on page complexity)
4. LLM reasons about what to click or extract
5. Performs an action
6. Takes another snapshot, sends it back
7. Repeats until done

For a simple task -- say, extracting the top stories from Hacker News
-- this might take 3-4 LLM round-trips and 5-15 seconds. Modern
accessibility-tree-based agents handle this reasonably well.

But the agent is rediscovering the same page structure every session.
HN's DOM hasn't changed. The CSS class for story titles is the same
today as yesterday. The agent doesn't need intelligence to find
`.titleline > a` -- it needs intelligence to decide *what to do with
the results*.

For tasks the agent performs repeatedly, the exploration overhead adds
up: extra latency, extra tokens, and occasional flaky results when the
LLM misidentifies an element.

## The Idea: Recipes as Pre-compiled Browser Knowledge

A recipe is a YAML file that encodes "how to do X in a browser" as a
deterministic script. The selectors, wait conditions, extraction logic,
and output format are all specified ahead of time.

```yaml
# hackernews-feed.yaml
name: "Hacker News Feed"
type: query
params:
  max_stories: { type: integer, default: "30" }
scripts:
  extract_stories: |
    (() => {
      const rows = document.querySelectorAll('.athing');
      return JSON.stringify([...rows].map(row => {
        const titleEl = row.querySelector('.titleline > a');
        const subtext = row.nextElementSibling;
        return {
          title: titleEl?.textContent?.trim(),
          url: titleEl?.href,
          points: parseInt(subtext?.querySelector('.score')?.textContent) || 0,
          comments: parseInt(subtext?.querySelectorAll('a').pop()?.textContent) || 0
        };
      }));
    })()
steps:
  - goto: "https://news.ycombinator.com"
    wait_for: ".athing"
  - eval: { ref: extract_stories, save_as: stories }
  - output: { stories: "{{ stories }}" }
```

The agent calls one function:

```
pwright script run hackernews-feed.yaml
```

It gets back structured JSON in 2-3 seconds. No snapshots sent to the
LLM. No reasoning about page structure. The recipe already knows where
the data is.

## Compile-time vs Runtime

This is the same trade-off as compiled vs interpreted code.

**Runtime exploration** (current agent approach):
```
Agent receives task
  -> navigate to site
  -> snapshot to LLM (accessibility tree or screenshot)
  -> LLM reasons about page structure
  -> perform action
  -> snapshot to LLM again
  -> LLM extracts data
  -> repeat N times
```

Each round-trip costs tokens and latency. The agent re-learns the same
page structure every session.

**Pre-compiled recipe** (our approach):
```
Agent receives task
  -> pick recipe + fill params (1 LLM call)
  -> execute recipe (deterministic, no LLM)
  -> receive structured JSON
  -> LLM interprets results (1 LLM call)
```

The browser interaction itself is deterministic -- JavaScript selectors
instead of LLM reasoning. The LLM is still involved in selecting the
recipe and interpreting results, but not in navigating the page.

| | Runtime exploration | Pre-compiled recipe |
|---|---|---|
| Latency | 5-30s (depends on steps) | 2-5s |
| LLM calls per action | 2-6+ round-trips | 1 (recipe selection) |
| Reliability | Good on simple pages, degrades on complex ones | Deterministic (same selectors every time) |
| Failure mode | Ambiguous (did the LLM extract the right thing?) | Explicit (selector not found = clear error) |
| Works on novel sites | Yes | No (needs a recipe first) |

The last row is important. Recipes don't replace runtime exploration --
they make it the fallback instead of the default path for known tasks.

## Why This Is Better Than It Sounds

### "This is just scripting, not AI"

Exactly right. The browser interaction part *should* be scripting.
Clicking a CSS selector is not a task that benefits from intelligence.
The agent's intelligence should be spent on:

- Deciding *which* recipe to call based on the user's request
- Interpreting the structured output
- Chaining recipes together for complex workflows
- Falling back to runtime exploration for sites without recipes

An agent figuring out how to click the right button on a page it's
visited 100 times is wasted effort. That's plumbing, not reasoning.

### "Selectors break when sites update"

They do. This is a real maintenance cost, and for sites with unstable
DOM structures it can outweigh the benefit of recipes.

But the failure mode is better:

- **Without recipes:** The agent might extract stale or wrong data
  without realizing it. Success depends on whether the LLM correctly
  interprets the changed page. Sometimes it does, sometimes it doesn't.

- **With recipes:** A changed DOM produces a clear error
  (`Element not found for selector: .athing`). You fix one YAML file,
  and every agent session works again.

In practice, recipes work best for sites with stable markup: internal
tools, documentation sites, well-structured public pages like HN.
They're a poor fit for sites that redesign frequently.

### "Why not just use APIs?"

APIs are better when they exist. Use them. Seriously. A proper API
will always be more reliable, faster, and better supported than
browser automation.

But APIs don't exist for everything:

- Many internal tools (dashboards, admin panels, legacy systems) have
  no API at all.
- Some data is only available in the rendered page (charts, computed
  layouts, visual state).
- Setting up OAuth, API keys, and permissions is sometimes more work
  than writing a 20-line recipe for a quick one-off task.

Recipes cover the gap between "there's a perfect API" and "the agent
has to figure it out from pixels."

### "Puppeteer/Selenium scripts have existed forever"

Yes. The scripting itself isn't new. What's new is the integration
point: scripts as *tools that agents call*, with structured I/O that
LLMs can reason about.

A Puppeteer script is a Node.js program that a developer runs. A
recipe is a parameterized function that an agent invokes. The
difference:

- **Params in, JSON out.** The agent doesn't need to understand
  JavaScript, CSS selectors, or browser automation. It fills in
  `query="rust browser automation"` and gets back a JSON array.
- **Discoverable.** Recipes have names, descriptions, typed params, and
  query/mutation tags. An agent can list available recipes and pick the
  right one.
- **Safe by default.** Query recipes can't cause side effects. Mutation
  recipes require confirmation. A raw Puppeteer script has no such
  guardrails.

The recipe format turns browser scripts into the same kind of
structured tool that function-calling APIs provide.

### "MCP already solves this"

MCP (Model Context Protocol) is a transport layer -- it defines how
agents discover and call tools. Recipes are the content that MCP
serves. They're complementary:

```
User -> Agent -> MCP -> pwright recipe -> Browser -> Structured data
```

An MCP server backed by recipes gives agents deterministic browser
tools. Without recipes, the MCP server would need to expose raw browser
control (screenshot, click, type), and the agent is back to runtime
exploration.

## Recipe Design Principles

### Structured output over raw HTML

Recipes return JSON, not DOM dumps. The agent never sees raw HTML.

```json
{
  "stories": [
    {"title": "Show HN: pwright", "url": "https://...", "points": 142, "comments": 57},
    {"title": "Rust in production", "url": "https://...", "points": 89, "comments": 31}
  ]
}
```

This is what the agent reasons about. Not `<tr class="athing">`.

### Query vs Mutation separation

Every recipe is tagged `query` (read-only) or `mutation` (has side
effects). Mutations require explicit confirmation (`--confirm`).

An agent can freely call query recipes for research. It needs user
approval before calling mutation recipes that send emails or submit
forms. This is a guardrail that runtime exploration lacks entirely --
when an agent clicks around a page, there's no way to distinguish
"reading" from "submitting."

### One JS eval over many DOM queries

Recipes prefer a single JavaScript evaluation that returns structured
data over multiple CDP round-trips. One `eval` that extracts 30 story
rows is faster and more reliable than 30 separate
`querySelector` + `getTextContent` calls.

### Wait for stability

Complex SPAs need time to hydrate after navigation. Recipes include
explicit `wait` steps and `wait_for` selectors tuned for each site.
An agent doing runtime exploration can also wait, but it has to guess
or use generic timeouts. A recipe encodes the right wait strategy
for each specific page.

## What We Ship

22 YAML recipes across 7 categories, ready to use. See
[recipe-catalog.md](knowledge/recipe-catalog.md) for the full list
including Chinese sites (Zhihu, Weibo, Xueqiu).

| Category | Subcategory | Recipes | Auth |
|----------|-------------|---------|:----:|
| Research | News | HN Feed | No |
| Research | Social | Reddit Feed, Reddit Search, X Feed, X Search, Weibo Feed, Weibo Hot | Mixed |
| Research | Search | Google Search | No |
| Research | Knowledge | GitHub Trending, Wikipedia Article | No |
| Research | Forums | Zhihu Hot, Zhihu Feed | Yes |
| Research | Finance | Xueqiu Quote | No |
| Research | Generic | Article Extractor, Table Extractor | No |
| Monitoring | -- | Dashboard Screenshot, Deploy Health Check | No |
| Communication | -- | Gmail Read, Gmail Send, GitHub Notifications | Yes |
| Automation | -- | Login + Extract, Form Fill + Submit | Yes |

```bash
# Extract HN front page (no auth, stable selectors)
pwright script run recipes/research/news/hackernews-feed.yaml

# Screenshot an internal dashboard
pwright script run recipes/monitoring/screenshot-dashboard.yaml \
  --param url="https://grafana.internal/d/api-latency"

# Extract a table from any page
pwright script run recipes/research/generic/extract-table.yaml \
  --param url="https://example.com/pricing" --param table_selector=".pricing-table"
```

## Writing New Recipes

Recipes are YAML files. A minimal recipe:

```yaml
name: "Extract page title"
type: query
params:
  url: { type: string, required: true }
steps:
  - goto: "{{ url }}"
  - extract: { selector: "h1", field: text_content, save_as: title }
  - output: { title: "{{ title }}" }
```

The real work is in the `scripts` section -- JavaScript that knows how
to extract structured data from a specific site's DOM. This is the
"compiled knowledge" that saves the agent from figuring it out at
runtime.

For complex sites, use the exploration-first workflow:

1. Open the site manually in Chrome
2. Inspect the DOM to find stable selectors
3. Write extraction JS and test it in the console
4. Package it as a recipe with params, waits, and error handling
5. Validate: `pwright script validate recipe.yaml`

This is a one-time cost per site. Every agent session after that
is free.

## The Self-Improving Loop: Agents Writing Recipes

The most interesting implication: agents can write recipes for
themselves.

When an agent encounters a site without a recipe, it falls back to
runtime exploration. But that exploration *produces knowledge* -- which
selectors worked, what data was extracted, how long waits needed to be.
An agent can package that knowledge into a recipe for next time.

```
First encounter:  Runtime exploration (slower, more tokens)
                  -> Agent writes extract-dashboard.yaml
Second encounter: pwright script run extract-dashboard.yaml (fast, deterministic)
```

This is caching, but for browser interaction knowledge. The cache key
is "how to read this dashboard." The cache value is a YAML file with
selectors and extraction logic.

The agent doesn't need to write perfect recipes. A recipe that works
most of the time and fails loudly when it doesn't is easier to debug
than runtime exploration that silently returns wrong data. When the
recipe fails, the agent can fall back to exploration and update the
recipe with what it learns.

Over time, an agent's recipe library grows. Common tasks get faster.
Novel tasks still work (via exploration). The system gets better
the more you use it, without retraining the model.

## Limitations

**Recipes are not universal.** They work for known sites with
reasonably stable DOM structures. They don't work for:

- Sites you've never seen before (use runtime exploration)
- Sites behind CAPTCHAs or aggressive bot detection
- Highly dynamic SPAs where the DOM structure changes per-user
- Tasks that require visual reasoning (is this chart trending up?)

The right model is a tiered approach:

```
Task arrives
  -> Is there a recipe for this? -> Yes: use recipe (fast, cheap, reliable)
  -> No recipe? -> Can the agent figure it out? -> Runtime exploration (slow, expensive)
  -> Too complex? -> Ask the human
```

Recipes cover the 80% of browser tasks that are repetitive and
well-understood. The remaining 20% still needs an agent that can
see and reason about pages.

## Try It

pwright is open source: https://github.com/Shuozeli/pwright

```bash
cargo build --release
pwright script run examples/recipes/research/news/hackernews-feed.yaml
```

Recipes are YAML. Write one for your internal dashboard, your
CI pipeline, your team's Slack -- whatever your agent does
repeatedly. Then stop paying for the LLM to rediscover it.
