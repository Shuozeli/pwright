# pwright Recipes

Pre-built YAML automation scripts for common browser tasks.

## Quick Start

```bash
# Run a query recipe (safe, read-only)
pwright script run examples/recipes/research/search/google-search.yaml \
  --param query="rust browser automation"

# Run a mutation recipe (has side effects -- review before running)
pwright script run examples/recipes/communication/gmail-send.yaml \
  --param to="user@example.com" --param subject="Hello" --param body="Hi there"
```

## Recipe Types

### Query (safe, read-only)

These recipes only READ data. They never click submit buttons, send
messages, or modify state. Safe to run without confirmation.

Marked with `type: query` in the recipe header.

### Mutation (modifies state, requires confirmation)

These recipes WRITE data: send emails, post messages, submit forms,
click buttons that trigger side effects. Review the recipe parameters
carefully before running.

Marked with `type: mutation` in the recipe header.

## Recipe Index

### Research: News

| Recipe | Description | Auth |
|--------|-------------|:----:|
| [hackernews-feed.yaml](research/news/hackernews-feed.yaml) | Extract HN front page stories | No |

### Research: Social

| Recipe | Description | Auth |
|--------|-------------|:----:|
| [reddit-feed.yaml](research/social/reddit-feed.yaml) | Extract posts from any subreddit | No |
| [reddit-search.yaml](research/social/reddit-search.yaml) | Search Reddit with sort options | No |
| [x-feed.yaml](research/social/x-feed.yaml) | Extract X.com timeline tweets | Yes |
| [x-search.yaml](research/social/x-search.yaml) | Search X.com with tab selection | Yes |
| [weibo-feed.yaml](research/social/weibo-feed.yaml) | Weibo home timeline posts | Yes |
| [weibo-hot.yaml](research/social/weibo-hot.yaml) | Weibo hot search trending | Yes |

### Research: Search

| Recipe | Description | Auth |
|--------|-------------|:----:|
| [google-search.yaml](research/search/google-search.yaml) | Search Google, extract top results | No |

### Research: Knowledge

| Recipe | Description | Auth |
|--------|-------------|:----:|
| [github-trending.yaml](research/knowledge/github-trending.yaml) | Trending GitHub repositories | No |
| [wikipedia-article.yaml](research/knowledge/wikipedia-article.yaml) | Wikipedia article summary | No |

### Research: Forums

| Recipe | Description | Auth |
|--------|-------------|:----:|
| [zhihu-hot.yaml](research/forums/zhihu-hot.yaml) | Zhihu hot questions | Yes |
| [zhihu-feed.yaml](research/forums/zhihu-feed.yaml) | Zhihu recommendation feed | Yes |

### Research: Finance

| Recipe | Description | Auth |
|--------|-------------|:----:|
| [xueqiu-quote.yaml](research/finance/xueqiu-quote.yaml) | Xueqiu stock quote + stats | No |

### Research: Generic

| Recipe | Description | Auth |
|--------|-------------|:----:|
| [extract-article.yaml](research/generic/extract-article.yaml) | Extract article content from any URL | No |
| [extract-table.yaml](research/generic/extract-table.yaml) | Extract HTML table as JSON | No |

### Monitoring (all query)

| Recipe | Description | Auth |
|--------|-------------|:----:|
| [screenshot-dashboard.yaml](monitoring/screenshot-dashboard.yaml) | Screenshot any URL | Varies |
| [check-deploy.yaml](monitoring/check-deploy.yaml) | Verify URL health + expected content | No |

### Communication

| Recipe | Type | Description | Auth |
|--------|------|-------------|:----:|
| [gmail-read.yaml](communication/gmail-read.yaml) | query | Read unread emails | Yes |
| [gmail-send.yaml](communication/gmail-send.yaml) | **mutation** | Send an email | Yes |
| [github-notifications.yaml](communication/github-notifications.yaml) | query | Read notifications | Yes |

### Automation

| Recipe | Type | Description | Auth |
|--------|------|-------------|:----:|
| [login-and-extract.yaml](automation/login-and-extract.yaml) | query | Login + extract protected content | Yes |
| [fill-and-submit.yaml](automation/fill-and-submit.yaml) | **mutation** | Fill form + submit | Varies |

### Network Capture (shell scripts)

These use `pwright network-listen` + `network-get` to intercept API traffic.
Run in a terminal while interacting with the site.

| Recipe | Description | Auth |
|--------|-------------|:----:|
| [discover-api.sh](network/discover-api.sh) | Navigate a site, capture all XHR/Fetch calls | Varies |
| [capture-form-submit.sh](network/capture-form-submit.sh) | Fill + submit a form, capture the API request/response | Varies |
| [extract-api-responses.sh](network/extract-api-responses.sh) | Capture API responses matching a URL pattern | Varies |
| [page-resource-audit.sh](network/page-resource-audit.sh) | Audit all resources loaded by a page | No |

## Adding New Recipes

See [recipe-catalog.md](../../docs/knowledge/recipe-catalog.md) for the full
proposed list including planned Chinese site recipes (Zhihu, Weibo, Xueqiu, Baidu).
