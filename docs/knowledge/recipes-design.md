# Recipes Design

Pre-built YAML script templates for common browser automation tasks.
Recipes are generic, parameterized scripts that work out of the box
with `pwright script run`.

## Goals

1. **Zero-to-value in seconds** -- user copies a recipe, fills in params, runs it
2. **Generic over site-specific** -- recipes adapt via params, not hardcoded selectors
3. **Agent-friendly output** -- structured JSON that Claude/LLMs can parse
4. **Reliable** -- wait steps, error handling, retry-safe
5. **Composable** -- recipes can be chained (output of one feeds into another)

## Directory Structure

```
examples/recipes/
  README.md
  communication/
    gmail-read.yaml           # Read unread emails from Gmail
    gmail-send.yaml           # Compose and send an email
    slack-read.yaml           # Read messages from a Slack channel
    slack-send.yaml           # Send a message to a Slack channel
    github-notifications.yaml # Read GitHub notifications
  monitoring/
    screenshot-dashboard.yaml # Navigate + screenshot any URL
    check-deploy.yaml         # Verify URL returns expected content
    github-actions-status.yaml # Check CI pipeline status
  research/
    google-search.yaml        # Search Google, extract top N results
    hackernews-feed.yaml      # Extract HN front page stories
    extract-article.yaml      # Extract article content from any news URL
    extract-table.yaml        # Extract HTML table data as JSON
  automation/
    login-and-extract.yaml    # Generic login flow + extract protected page
    fill-form.yaml            # Generic form fill + submit
    download-file.yaml        # Navigate + click download link + save file
```

## Recipe Format

Each recipe follows a consistent structure:

```yaml
# recipe-name.yaml -- One-line description
#
# Usage: pwright script run examples/recipes/category/recipe.yaml \
#          --param key=value
#
# Prerequisites: (if any, e.g. "must be logged into Gmail in Chrome")
#
# Output: description of what the output JSON contains

name: "Human-readable name"
version: 1

params:
  required_param:
    type: string
    required: true
    description: "What this param controls"
  optional_param:
    type: string
    default: "sensible default"
    description: "Optional behavior tweak"

config:
  default_timeout_ms: 10000

scripts:
  extract_data: |
    // Centralized JS for data extraction
    // Returns JSON string for structured output

steps:
  - goto: "{{ url }}"
    wait_for: "selector"
  - wait: 2000                    # anti-bot / page settle
  - eval: { ref: extract_data, save_as: data }
  - output: { data: "{{ data }}" }
```

## Recipe Specifications

### Communication

#### gmail-read.yaml

Read unread emails from Gmail inbox.

**Prerequisites:** User must be logged into Gmail in the Chrome instance.
No OAuth needed -- uses existing browser session.

**Params:**
- `max_emails` (int, default: 10) -- how many to extract
- `label` (string, default: "inbox") -- Gmail label/folder

**Steps:**
1. Navigate to `https://mail.google.com/mail/u/0/#${label}`
2. Wait for inbox to load (`.zA` or similar inbox row selector)
3. Wait 2s for dynamic content
4. JS eval: extract sender, subject, snippet, date, unread status
5. Output as JSON array

**Output:**
```json
{
  "emails": [
    {"from": "alice@example.com", "subject": "Meeting tomorrow", "snippet": "Can we...", "date": "Mar 15", "unread": true},
    ...
  ]
}
```

**Challenges:**
- Gmail DOM changes frequently. Recipe may need periodic selector updates.
- Multiple account switcher (`/u/0/` vs `/u/1/`)
- Conversation view vs individual messages

#### gmail-send.yaml

Compose and send an email.

**Params:**
- `to` (string, required)
- `subject` (string, required)
- `body` (string, required)

**Steps:**
1. Navigate to Gmail
2. Click "Compose" button
3. Wait for compose window
4. Fill "To" field
5. Fill "Subject" field
6. Fill body
7. Click "Send"
8. Wait for sent confirmation

#### slack-read.yaml

Read recent messages from a Slack channel.

**Prerequisites:** Logged into Slack workspace in Chrome.

**Params:**
- `workspace_url` (string, required) -- e.g. `https://myteam.slack.com`
- `channel` (string, required) -- channel name
- `max_messages` (int, default: 20)

**Steps:**
1. Navigate to `${workspace_url}/archives/${channel}` (or channel URL)
2. Wait for messages to load
3. JS eval: extract message sender, text, timestamp
4. Output as JSON array

**Note:** Slack has excellent APIs and MCP integrations. This recipe is
for cases where API access is unavailable (external workspaces, free
tier limitations, Slack Connect channels).

#### github-notifications.yaml

Read unread GitHub notifications.

**Prerequisites:** Logged into GitHub in Chrome.

**Params:**
- `max_notifications` (int, default: 20)

**Steps:**
1. Navigate to `https://github.com/notifications`
2. Wait for notification list
3. JS eval: extract notification type, repo, title, time, unread status
4. Output as JSON array

### Monitoring

#### screenshot-dashboard.yaml

Navigate to any URL and take a screenshot. Generic -- works with any
dashboard (Grafana, Datadog, Vercel, custom).

**Params:**
- `url` (string, required)
- `wait_for` (string, default: "body") -- CSS selector to wait for
- `wait_ms` (int, default: 3000) -- wait after selector appears
- `filename` (string, default: "dashboard.png")

**Steps:**
1. Navigate to URL
2. Wait for selector
3. Additional wait for charts/graphs to render
4. Screenshot

**Output:** Screenshot file saved to disk.

#### check-deploy.yaml

Verify a deployment is healthy: URL loads, expected content present.

**Params:**
- `url` (string, required)
- `expected_text` (string, required) -- text that must be on the page
- `timeout_ms` (int, default: 10000)

**Steps:**
1. Navigate to URL
2. Wait for page load
3. Extract page content
4. Check if expected_text is present
5. Output: pass/fail with details

**Output:**
```json
{
  "url": "https://app.example.com",
  "status": "pass",
  "title": "My App",
  "expected_text_found": true,
  "load_time_ms": 1234
}
```

#### github-actions-status.yaml

Check the status of GitHub Actions workflows.

**Prerequisites:** Logged into GitHub.

**Params:**
- `repo` (string, required) -- e.g. "owner/repo"
- `workflow` (string, optional) -- specific workflow name

**Steps:**
1. Navigate to `https://github.com/${repo}/actions`
2. Wait for workflow list
3. JS eval: extract workflow name, status, duration, commit
4. Output as JSON array

### Research

#### google-search.yaml

Search Google and extract top results.

**Params:**
- `query` (string, required)
- `max_results` (int, default: 10)

**Steps:**
1. Navigate to `https://www.google.com/search?q=${query}`
2. Wait for results
3. JS eval: extract title, URL, snippet for each result
4. Output as JSON array

**Output:**
```json
{
  "query": "rust browser automation",
  "results": [
    {"title": "pwright - ...", "url": "https://...", "snippet": "..."},
    ...
  ]
}
```

#### hackernews-feed.yaml

Extract stories from Hacker News front page.

**Params:**
- `pages` (int, default: 1) -- number of pages to scrape
- `max_stories` (int, default: 30)

**Steps:**
1. Navigate to HN
2. JS eval: extract rank, title, URL, points, comments
3. If pages > 1: click "More", wait, extract next page
4. Output as JSON array

#### extract-article.yaml

Extract the main content from any article/blog URL.

**Params:**
- `url` (string, required)

**Steps:**
1. Navigate to URL
2. Wait for article content
3. JS eval: extract title, author, date, body text (heuristic: largest
   text block, or `<article>` tag, or `role="main"`)
4. Output as JSON

**Note:** This is intentionally heuristic. Works well for standard
article layouts. May need per-site customization for unusual designs.

#### extract-table.yaml

Extract HTML table data as structured JSON.

**Params:**
- `url` (string, required)
- `table_selector` (string, default: "table") -- CSS selector for the table

**Steps:**
1. Navigate to URL
2. Wait for table
3. JS eval: extract headers from `<th>`, rows from `<td>`, build array of objects
4. Output as JSON array of row objects

### Automation

#### login-and-extract.yaml

Generic login flow: fill credentials, submit, extract from protected page.

**Params:**
- `login_url` (string, required)
- `username` (string, required)
- `password` (string, required)
- `username_selector` (string, default: "#username")
- `password_selector` (string, default: "#password")
- `submit_selector` (string, default: "button[type=submit]")
- `target_url` (string, required) -- URL to visit after login
- `extract_selector` (string, required) -- what to extract

**Steps:**
1. Navigate to login URL
2. Fill username, fill password, click submit
3. Wait for redirect / dashboard
4. Navigate to target URL
5. Extract content from extract_selector
6. Output

#### fill-form.yaml

Generic form fill and submit. Fields specified as params.

**Params:**
- `url` (string, required)
- `fields` (string, required) -- JSON: `[{"selector":"#name","value":"John"},...]`
- `submit_selector` (string, default: "button[type=submit]")

**Steps:**
1. Navigate to URL
2. Wait for form
3. For each field: fill selector with value
4. Click submit
5. Wait for result
6. Output success/failure

## Implementation Priority

| Priority | Recipes | Rationale |
|----------|---------|-----------|
| P0 | google-search, hackernews-feed, screenshot-dashboard | Immediately useful, public sites, no auth |
| P0 | extract-table, extract-article | Generic, high reuse |
| P1 | gmail-read, github-notifications | Common agent tasks, require auth |
| P1 | check-deploy, login-and-extract | Operational value |
| P2 | gmail-send, slack-read, slack-send | Communication, fragile selectors |
| P2 | fill-form, download-file | Automation helpers |
| P3 | github-actions-status | Niche, GitHub API is better |

## Design Principles

### 1. Params over hardcoded selectors
Bad: `click: "#gbqfbb"` (Google's internal ID)
Good: `click: "{{ submit_selector }}"` with a sensible default

### 2. JS extraction over DOM queries
Prefer one JS eval that returns structured JSON over multiple extract
steps. JS can handle complex DOM traversal, filtering, and formatting
in one round trip.

### 3. Wait for stability
Always `wait` 1-3 seconds after navigation on complex sites. Dynamic
content (React, Angular) needs time to hydrate. Anti-bot systems
trigger on rapid sequential requests.

### 4. Graceful degradation
Use `on_error: continue` for optional elements. Not every page has
every element. A recipe should return partial results rather than
failing completely.

### 5. Output as JSON
All recipes output structured JSON, not raw text. This lets LLM agents
parse and reason about the results programmatically.

### 6. Document prerequisites
If a recipe requires login, say so clearly. Don't assume auth state.
The user must ensure Chrome has an active session before running
auth-dependent recipes.

## Recipes vs MCP

| Task | MCP available? | Recipe still useful? |
|------|---------------|---------------------|
| Slack messages | Yes (Slack MCP) | Only for Slack Connect / no API access |
| GitHub issues | Yes (GitHub MCP) | No -- use MCP |
| Gmail | Yes (Google API) | Yes -- browser auth is simpler for one-off reads |
| Google search | No good MCP | Yes -- primary use case |
| Dashboard screenshots | No | Yes -- primary use case |
| Form filling | No | Yes -- primary use case |
| Web scraping | No | Yes -- primary use case |

Recipes complement MCP -- they cover the gap where no API/MCP exists
or where browser-level auth is simpler than setting up API credentials.

## Phase 1 Deliverables

Build and test 6 recipes:
1. `google-search.yaml`
2. `hackernews-feed.yaml`
3. `screenshot-dashboard.yaml`
4. `extract-table.yaml`
5. `extract-article.yaml`
6. `check-deploy.yaml`

These require no authentication and work on public sites. They can be
tested in CI against the integration test server (for extract-table,
check-deploy) and manually against real sites (Google, HN).
