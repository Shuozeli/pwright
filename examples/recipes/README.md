# pwright Recipes

Pre-built YAML automation scripts for common browser tasks.

## Quick Start

```bash
# Run a query recipe (safe, read-only)
pwright script run examples/recipes/research/google-search.yaml \
  --param query="rust browser automation"

# Run a mutation recipe (requires --confirm flag)
pwright script run examples/recipes/communication/gmail-send.yaml \
  --param to="user@example.com" --param subject="Hello" --param body="Hi there" \
  --confirm
```

## Recipe Types

### Query (safe, read-only)

These recipes only READ data. They never click submit buttons, send
messages, or modify state. Safe to run without confirmation.

Marked with `type: query` in the recipe header.

### Mutation (modifies state, requires confirmation)

These recipes WRITE data: send emails, post messages, submit forms,
click buttons that trigger side effects. They require the `--confirm`
flag to execute.

Marked with `type: mutation` in the recipe header.

## Recipe Index

### Research (all query)

| Recipe | Description | Auth required |
|--------|-------------|:------------:|
| [google-search.yaml](research/google-search.yaml) | Search Google, extract top results | No |
| [hackernews-feed.yaml](research/hackernews-feed.yaml) | Extract HN front page stories | No |
| [extract-article.yaml](research/extract-article.yaml) | Extract article content from any URL | No |
| [extract-table.yaml](research/extract-table.yaml) | Extract HTML table as JSON | No |

### Monitoring (all query)

| Recipe | Description | Auth required |
|--------|-------------|:------------:|
| [screenshot-dashboard.yaml](monitoring/screenshot-dashboard.yaml) | Screenshot any URL | Depends |
| [check-deploy.yaml](monitoring/check-deploy.yaml) | Verify URL health + expected content | No |

### Communication

| Recipe | Type | Description | Auth required |
|--------|------|-------------|:------------:|
| [gmail-read.yaml](communication/gmail-read.yaml) | query | Read unread emails | Yes |
| [gmail-send.yaml](communication/gmail-send.yaml) | **mutation** | Send an email | Yes |
| [github-notifications.yaml](communication/github-notifications.yaml) | query | Read notifications | Yes |

### Automation

| Recipe | Type | Description | Auth required |
|--------|------|-------------|:------------:|
| [login-and-extract.yaml](automation/login-and-extract.yaml) | query | Login + extract protected content | Yes |
| [fill-and-submit.yaml](automation/fill-and-submit.yaml) | **mutation** | Fill form + submit | Depends |
